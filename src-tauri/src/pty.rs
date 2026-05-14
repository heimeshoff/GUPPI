//! Claude session ownership & PTY — the ADR-006 empirical spike
//! (`infrastructure-013-pty-spike`).
//!
//! ADR-006 decided GUPPI spawns the native Windows `claude.exe` through
//! `portable-pty` (ConPTY), one **actor** per session, each child wrapped in a
//! Windows **Job Object** configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
//! so no orphan `claude.exe` survives a GUPPI crash or exit. PTY is the
//! riskiest piece of the architecture; ADR-006 deferred the empirical proof to
//! this spike because no Rust scaffold existed when the ADR was written. The
//! walking skeleton (`infrastructure-012`) created that scaffold; this module
//! is the deferred proof.
//!
//! ## What this module proves
//!
//! - `claude.exe` spawns through `portable-pty` with `cwd` set per-spawn and
//!   the parent environment inherited (ADR-006's "cwd-per-spawn").
//! - The child is wrapped in a Windows Job Object with
//!   `KILL_ON_JOB_CLOSE` — when the `ClaudeSession` is dropped (clean exit) or
//!   when GUPPI crashes (the OS closes the handle), the OS tears the whole job
//!   down. No orphans.
//! - A **read loop** pulls raw bytes off the PTY master and publishes
//!   `SessionOutput` onto the ADR-009 `EventBus`. No VT parsing — raw bytes per
//!   ADR-006 ("the read loop ships raw bytes until the terminal panel lands").
//! - A **write channel** feeds input to the child.
//! - A **resize channel** changes the terminal size without crashing.
//!
//! ## Actor boundary (ADR-006 "Reversibility")
//!
//! The rest of GUPPI talks to a `ClaudeSession` only through `write`,
//! `resize`, `is_alive`, and the `SessionOutput` events on the bus — never to
//! `portable-pty` directly. Swapping the PTY library, or reacting to a failed
//! spike, is contained behind this boundary.
//!
//! ## Scope
//!
//! Windows-only day one (ADR-006 / ADR-001). The Job Object wiring is
//! `#[cfg(windows)]`; on other platforms `portable-pty` still spawns over
//! `openpty` but no job wrapping happens — those paths are *not validated*.

use crate::events::{DomainEvent, EventBus};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Default PTY size a session starts at. ADR-006 leaves sizing to the
/// terminal-panel feature; the spike just needs a sane initial value and a
/// resize that does not crash.
pub const DEFAULT_PTY_ROWS: u16 = 24;
pub const DEFAULT_PTY_COLS: u16 = 80;

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("the spawn cwd does not exist or is not a directory: {0}")]
    CwdMissing(String),
    #[error("could not open a pty: {0}")]
    OpenPty(String),
    #[error("could not spawn the child process: {0}")]
    Spawn(String),
    #[error("could not take a handle to the pty: {0}")]
    Handle(String),
    #[error("windows job object error: {0}")]
    JobObject(String),
    #[error("io error talking to the pty: {0}")]
    Io(#[from] std::io::Error),
}

/// A live `claude.exe` session — the ADR-006 actor.
///
/// Owns the PTY master/slave pair, the child process, the read-loop thread,
/// and (on Windows) the Job Object the child runs inside. Dropping this value
/// is the cleanup path: it signals the read loop to stop, kills the child, and
/// closes the Job Object handle — which, with `KILL_ON_JOB_CLOSE`, guarantees
/// the OS reaps the whole process tree.
pub struct ClaudeSession {
    /// Stable id for this session — used as the `project_id`-equivalent on
    /// `SessionOutput` events so a consumer can route output. The spike uses a
    /// caller-supplied id.
    session_id: i64,
    /// The writer half of the PTY master — input goes here. `Option` so `Drop`
    /// can release it *before* joining the read loop: on Windows the ConPTY
    /// reader does not see EOF until the master handles are closed, so the
    /// master and writer must be dropped first or the join would hang.
    writer: Option<Box<dyn Write + Send>>,
    /// The master handle, kept for `resize`. `Option` for the same
    /// drop-ordering reason as `writer`.
    master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    /// The spawned child. `Some` until killed.
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Flips to `false` to ask the read-loop thread to exit.
    read_loop_alive: Arc<AtomicBool>,
    /// Join handle for the read-loop thread; joined (best-effort, bounded) on
    /// drop.
    read_loop: Option<JoinHandle<()>>,
    /// Windows Job Object the child runs inside. Dropping it closes the
    /// handle; with `KILL_ON_JOB_CLOSE` that kills the whole tree.
    #[cfg(windows)]
    _job: job::JobObject,
}

impl ClaudeSession {
    /// Spawn `claude.exe` in `cwd`, inheriting GUPPI's environment, wrapped in
    /// a Windows Job Object, with a read loop publishing `SessionOutput` onto
    /// `bus`.
    ///
    /// `program` is the executable to run. In production this is `claude.exe`;
    /// the spike's automated tests pass a cheap, deterministic stand-in (e.g.
    /// `cmd.exe`) so CI does not depend on a real Claude login — the *PTY +
    /// Job Object + cwd + env* mechanics are identical regardless of which
    /// program runs inside.
    pub fn spawn(
        session_id: i64,
        program: &str,
        args: &[&str],
        cwd: &Path,
        bus: EventBus,
    ) -> Result<Self, PtyError> {
        if !cwd.is_dir() {
            return Err(PtyError::CwdMissing(cwd.to_string_lossy().into_owned()));
        }

        // --- open the PTY (ConPTY on Windows via portable-pty) -------------
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: DEFAULT_PTY_ROWS,
                cols: DEFAULT_PTY_COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::OpenPty(e.to_string()))?;

        // --- build the command: cwd-per-spawn + env inheritance -----------
        // ADR-006: `cwd = project.path`, environment inherited from GUPPI's
        // process. `CommandBuilder` inherits the parent env by default; we set
        // cwd explicitly.
        let mut cmd = CommandBuilder::new(program);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.cwd(cwd);

        // --- spawn the child onto the PTY slave ---------------------------
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        // --- ADR-006: wrap the child in a Windows Job Object --------------
        // KILL_ON_JOB_CLOSE means the OS reaps the whole tree when the job
        // handle closes — on clean drop *and* on a GUPPI crash. This is the
        // deliberate wiring that replaces Unix process-group semantics.
        #[cfg(windows)]
        let _job = {
            let job = job::JobObject::new().map_err(PtyError::JobObject)?;
            if let Some(pid) = child.process_id() {
                job.assign_process_by_pid(pid).map_err(PtyError::JobObject)?;
            } else {
                tracing::warn!(
                    session_id,
                    "child exposed no pid; cannot assign to job object"
                );
            }
            job
        };

        // --- the read loop: raw bytes -> SessionOutput on the bus ---------
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Handle(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Handle(e.to_string()))?;

        let read_loop_alive = Arc::new(AtomicBool::new(true));
        let loop_flag = read_loop_alive.clone();
        let read_loop = std::thread::Builder::new()
            .name(format!("claude-session-{session_id}-read"))
            .spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    if !loop_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            // EOF — the child closed the PTY. Done.
                            tracing::info!(session_id, "pty reached EOF; read loop exiting");
                            break;
                        }
                        Ok(n) => {
                            // ADR-006: ship raw bytes, no VT parsing yet.
                            bus.publish(DomainEvent::SessionOutput {
                                session_id,
                                bytes: buf[..n].to_vec(),
                            });
                        }
                        Err(e) => {
                            // On Windows, a closed ConPTY surfaces as an error
                            // rather than a clean 0; treat it as end-of-stream.
                            tracing::info!(
                                session_id,
                                error = %e,
                                "pty read ended; read loop exiting"
                            );
                            break;
                        }
                    }
                }
            })
            .map_err(PtyError::Io)?;

        tracing::info!(
            session_id,
            program,
            cwd = %cwd.display(),
            "claude session spawned"
        );

        Ok(Self {
            session_id,
            writer: Some(writer),
            master: Some(pair.master),
            child,
            read_loop_alive,
            read_loop: Some(read_loop),
            #[cfg(windows)]
            _job,
        })
    }

    /// Session id this actor was spawned with. Exercised by the spike tests
    /// (the IPC layer uses a fixed id); kept on the public actor surface
    /// because multi-session work will route by it.
    #[allow(dead_code)]
    pub fn session_id(&self) -> i64 {
        self.session_id
    }

    /// Feed input bytes to the child through the PTY master (the ADR-006
    /// "write channel", in its simplest synchronous form for the spike).
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| PtyError::Handle("session is being torn down".into()))?;
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }

    /// Resize the terminal (the ADR-006 "resize channel"). The spike's DoD only
    /// requires that this does not crash the session.
    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), PtyError> {
        let master = self
            .master
            .as_ref()
            .ok_or_else(|| PtyError::Handle("session is being torn down".into()))?;
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Handle(e.to_string()))
    }

    /// Whether the child is still running. Non-blocking.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Kill the child explicitly. Drop also does this; this is the explicit
    /// path for a deliberate "end session" command. The IPC `pty_kill`
    /// command ends a session by dropping it (which runs the same teardown);
    /// this explicit method is exercised by the spike tests.
    #[allow(dead_code)]
    pub fn kill(&mut self) -> Result<(), PtyError> {
        self.child
            .kill()
            .map_err(|e| PtyError::Spawn(e.to_string()))?;
        Ok(())
    }
}

impl Drop for ClaudeSession {
    /// ADR-006 "Cleanup on drop". The deterministic teardown:
    ///
    /// 1. Signal the read loop to stop and kill the child.
    /// 2. **Drop the PTY master + writer.** On Windows the ConPTY reader does
    ///    not observe EOF until the master handles close — so this must happen
    ///    *before* the join, or the join would block until the OS lazily tore
    ///    the pseudoconsole down.
    /// 3. Join the read-loop thread — but only *best-effort and bounded*: a
    ///    session teardown must never hang GUPPI. The Job Object is the real
    ///    cleanup guarantee; the join is just tidiness.
    /// 4. Let `_job` drop — closing the handle triggers `KILL_ON_JOB_CLOSE`,
    ///    so the OS reaps the whole process tree (orphan-free).
    fn drop(&mut self) {
        self.read_loop_alive.store(false, Ordering::Relaxed);
        if let Err(e) = self.child.kill() {
            tracing::warn!(
                session_id = self.session_id,
                error = %e,
                "failed to kill child on drop"
            );
        }
        let _ = self.child.wait();

        // Release the PTY master + writer so the reader thread sees EOF.
        self.writer.take();
        self.master.take();

        if let Some(handle) = self.read_loop.take() {
            // Bounded best-effort join: hand the join to a watchdog thread and
            // wait at most briefly. If the reader is still parked, we detach —
            // the thread dies on its own once ConPTY fully closes, and the
            // Job Object has already guaranteed no orphan processes.
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = handle.join();
                let _ = tx.send(());
            });
            if rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .is_err()
            {
                tracing::warn!(
                    session_id = self.session_id,
                    "read loop did not stop within 2s; detaching (job object still guarantees cleanup)"
                );
            }
        }
        tracing::info!(session_id = self.session_id, "claude session cleaned up");
        // `_job` drops here on Windows -> handle closes -> OS reaps the tree.
    }
}

/// Windows Job Object wrapper — ADR-006's orphan-free cleanup mechanism.
#[cfg(windows)]
mod job {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
        JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    /// An owned Windows Job Object handle. Created with
    /// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: when this handle closes — on a
    /// clean `Drop` or because the OS tore down a crashed GUPPI — every
    /// process assigned to the job is killed. No orphan `claude.exe`.
    pub struct JobObject {
        handle: HANDLE,
    }

    // The handle is just an owned OS resource; it is safe to move across
    // threads (the actor lives on a Tokio task in production).
    unsafe impl Send for JobObject {}
    unsafe impl Sync for JobObject {}

    impl JobObject {
        /// Create a job object configured to kill its processes when the
        /// handle closes.
        pub fn new() -> Result<Self, String> {
            // SAFETY: a single FFI call; we own the returned handle and check
            // it for validity immediately.
            let handle = unsafe { CreateJobObjectW(None, None) }
                .map_err(|e| format!("CreateJobObjectW failed: {e}"))?;

            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            // SAFETY: `info` is a correctly-sized, fully-initialised struct of
            // the type named by `JobObjectExtendedLimitInformation`; `handle`
            // is valid (checked above).
            let ok = unsafe {
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const core::ffi::c_void,
                    core::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if let Err(e) = ok {
                // SAFETY: handle is valid; closing it on the error path.
                unsafe {
                    let _ = CloseHandle(handle);
                }
                return Err(format!("SetInformationJobObject failed: {e}"));
            }

            Ok(Self { handle })
        }

        /// Assign an already-running process (by pid) to this job. ConPTY
        /// spawns the child for us, so we adopt it by pid rather than creating
        /// it suspended.
        pub fn assign_process_by_pid(&self, pid: u32) -> Result<(), String> {
            // SAFETY: FFI; we request only the access rights needed to assign
            // the process to a job and let the job terminate it, and we close
            // the process handle as soon as the assignment is done.
            let process = unsafe {
                OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
            }
            .map_err(|e| format!("OpenProcess({pid}) failed: {e}"))?;

            // SAFETY: both handles are valid.
            let result = unsafe { AssignProcessToJobObject(self.handle, process) };

            // SAFETY: closing the process handle we just opened; the job keeps
            // its own reference to the process.
            unsafe {
                let _ = CloseHandle(process);
            }

            result.map_err(|e| format!("AssignProcessToJobObject({pid}) failed: {e}"))
        }
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            // SAFETY: `handle` was created by `CreateJobObjectW` and not yet
            // closed. Closing it triggers KILL_ON_JOB_CLOSE.
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{DomainEvent, EventBus};
    use std::time::Duration;

    /// A cheap, always-available stand-in for `claude.exe`. The PTY + Job
    /// Object + cwd + env mechanics this spike validates are identical
    /// regardless of which program runs inside the PTY, and CI must not depend
    /// on a real Claude login.
    #[cfg(windows)]
    const TEST_PROGRAM: &str = "cmd.exe";
    #[cfg(not(windows))]
    const TEST_PROGRAM: &str = "sh";

    fn collect_output(rx: &mut tokio::sync::broadcast::Receiver<DomainEvent>, within: Duration) -> Vec<u8> {
        let deadline = std::time::Instant::now() + within;
        let mut out = Vec::new();
        while std::time::Instant::now() < deadline {
            match rx.try_recv() {
                Ok(DomainEvent::SessionOutput { bytes, .. }) => out.extend_from_slice(&bytes),
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(_) => break,
            }
        }
        out
    }

    #[test]
    fn refuses_to_spawn_in_a_missing_cwd() {
        let bus = EventBus::new();
        let missing = std::env::temp_dir().join("guppi-pty-no-such-dir-xyz");
        let _ = std::fs::remove_dir_all(&missing);
        match ClaudeSession::spawn(1, TEST_PROGRAM, &[], &missing, bus) {
            Err(PtyError::CwdMissing(_)) => {}
            Err(other) => panic!("expected CwdMissing, got {other:?}"),
            Ok(_) => panic!("expected CwdMissing, got a live session"),
        }
    }

    /// Proves: spawn through `portable-pty` in a chosen `cwd`, the read loop
    /// streams the child's output back onto the bus as `SessionOutput`. This
    /// is DoD criterion 1 (minus the "is literally claude.exe" part, which is
    /// a hands-on check) and the read-loop half of criterion 3.
    #[test]
    fn spawns_in_cwd_and_streams_output_to_the_bus() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let cwd = std::env::temp_dir();

        // `cmd /C cd` prints the current directory — proves cwd-per-spawn.
        #[cfg(windows)]
        let (program, args) = ("cmd.exe", vec!["/C", "cd"]);
        #[cfg(not(windows))]
        let (program, args) = ("pwd", vec![]);

        let mut session =
            ClaudeSession::spawn(10, program, &args, &cwd, bus).expect("session should spawn");
        assert_eq!(session.session_id(), 10);

        let output = collect_output(&mut rx, Duration::from_secs(5));
        let text = String::from_utf8_lossy(&output);
        assert!(
            !output.is_empty(),
            "the read loop should have streamed the child's output onto the bus"
        );

        // The child ran in `cwd`; its printed directory should reflect that.
        let cwd_leaf = cwd
            .file_name()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if !cwd_leaf.is_empty() {
            assert!(
                text.to_lowercase().contains(&cwd_leaf),
                "child output {text:?} should mention the spawn cwd leaf {cwd_leaf:?}"
            );
        }

        let _ = session.kill();
    }

    /// Proves: input written to the PTY round-trips to the child and the
    /// child's response comes back on the bus; a resize does not crash the
    /// session. DoD criterion 2.
    #[test]
    fn input_and_resize_round_trip_without_crashing() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let cwd = std::env::temp_dir();

        let mut session = ClaudeSession::spawn(20, TEST_PROGRAM, &[], &cwd, bus)
            .expect("session should spawn");

        // Resize first — must not crash the still-young session.
        session.resize(40, 120).expect("resize should not error");
        assert!(session.is_alive(), "session should survive a resize");

        // Drain the shell banner so the marker is easy to find afterwards.
        let _ = collect_output(&mut rx, Duration::from_millis(500));

        // Write a command whose echo we can recognise in the output stream.
        #[cfg(windows)]
        let line = b"echo guppi_pty_marker\r\n";
        #[cfg(not(windows))]
        let line = b"echo guppi_pty_marker\n";
        session.write(line).expect("write should succeed");

        let output = collect_output(&mut rx, Duration::from_secs(5));
        let text = String::from_utf8_lossy(&output);
        assert!(
            text.contains("guppi_pty_marker"),
            "the child should have reacted to written input; got {text:?}"
        );

        // Resize again after activity — still must not crash.
        session.resize(24, 80).expect("second resize should not error");
        assert!(session.is_alive(), "session should survive a second resize");

        let _ = session.kill();
    }

    /// Proves: dropping the `ClaudeSession` actually tears the child down — the
    /// drop-path cleanup ADR-006 specifies, and the programmatic half of DoD
    /// criterion 4 ("killing GUPPI normally leaves zero orphans"). The
    /// force-crash half (criterion 5) needs the Job Object and Task Manager —
    /// a hands-on check recorded in the task Outcome.
    #[test]
    fn dropping_the_session_kills_the_child() {
        let bus = EventBus::new();
        let cwd = std::env::temp_dir();

        let pid;
        {
            let session = ClaudeSession::spawn(30, TEST_PROGRAM, &[], &cwd, bus)
                .expect("session should spawn");
            // `cmd.exe` with no args sits idle waiting for input — a good
            // stand-in for a long-lived TUI that will not exit on its own.
            pid = child_pid_of(&session);
            assert!(pid.is_some(), "spawned child should expose a pid");
        } // <- Drop runs here: read loop stopped, child killed, job closed.

        if let Some(pid) = pid {
            // Give the OS a beat to reap.
            std::thread::sleep(Duration::from_millis(500));
            assert!(
                !pid_is_running(pid),
                "child pid {pid} should be gone after the session was dropped"
            );
        }
    }

    /// Test-only peek at the child pid for the orphan check.
    fn child_pid_of(session: &ClaudeSession) -> Option<u32> {
        session.child.process_id()
    }

    /// Cross-checks whether a pid is still running, using the OS process list.
    #[cfg(windows)]
    fn pid_is_running(pid: u32) -> bool {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output();
        match output {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                text.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }

    #[cfg(not(windows))]
    fn pid_is_running(pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}
