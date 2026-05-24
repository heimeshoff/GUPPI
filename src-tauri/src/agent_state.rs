//! Per-task live agent-state projection — `agent-awareness-002`.
//!
//! agent-awareness's job is a single unified "what's running, what's idle, what
//! needs an answer" view that survives its two signal sources (rich
//! `claude-runner` events for owned sessions; best-effort filesystem signals for
//! observed sessions — see the BC README). The canvas-accordion pivot moved the
//! granularity from per-BC to **per-task**: a card shows "orchestrator · waiting
//! 2m 14s" and the docked panel renders an "AGENT NEEDS AN ANSWER" callout when
//! the task is blocked.
//!
//! This module owns the **read side** (v1 is read-only per the vision — ADR-018):
//!
//! - [`TaskAgentState`] — the projected state for one `(project_id, bc, task_id)`:
//!   `running | idle | blocked_on_question`, plus, for running/blocked, the acting
//!   agent label and the *transition timestamp* (`since`). The card derives the
//!   "waiting 2m 14s" elapsed string from `since`; this BC never spams a
//!   per-second event.
//! - [`AgentStateProjection`] — the in-core projection keyed by
//!   `(project_id, bc, task_id)`. It ingests signals from both sources via
//!   [`AgentStateProjection::ingest`] and folds them into the unified model. The
//!   source split lives behind a single `ingest`; the canvas sees one shape.
//! - [`BcRollup`] — the per-BC `active / blocked / idling` counts the accordion
//!   header renders ("1 active · 2 blocked · 1 idling"), derived from the
//!   per-task states.
//!
//! ## Source split (ADR-018)
//!
//! `claude-runner` (owned sessions) is the **rich** source: it emits
//! `SessionBlockedOnQuestion { project_id, bc, task_id, agent_label, question }`
//! carrying the live question text the callout renders. The filesystem
//! (observed sessions) is the **best-effort** source: a task present in `doing/`
//! with a `blocked_question:` in its on-disk frontmatter is a lower-fidelity
//! blocked signal (the disk question is the *fallback* per
//! `project-registry-005`). Both fold into the same [`TaskAgentState`].
//!
//! v1 ships the runner path as the primary live source and the filesystem path
//! as the fallback fidelity; the answer / defer / edit **write** round-trip is
//! out of scope (read-only v1 — the panel renders the actions visual-only, the
//! command path is post-v1).

use crate::events::DomainEvent;
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The unified per-task agent state — `running | idle | blocked_on_question`.
///
/// Serialised `snake_case` to match the on-the-wire vocabulary the canvas
/// already consumes (ADR-009 / `project-registry-005`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActivity {
    /// At least one session is actively producing output for this task.
    Running,
    /// No active session for this task, or the active session is waiting on
    /// nothing. The default state for a task with no live signal.
    Idle,
    /// An active session is waiting for human input. Carries the question text
    /// the docked panel's "AGENT NEEDS AN ANSWER" callout renders.
    BlockedOnQuestion,
}

/// The projected live state for one task, keyed externally by
/// `(project_id, bc, task_id)`. The canvas reads this to draw the per-card
/// agent indicator and (for blocked tasks) the docked-panel callout.
///
/// `agent_label` and `since` are present for `running` / `blocked_on_question`
/// and `None` for `idle`. `question` is present only for `blocked_on_question`.
///
/// `since` is a Unix-millisecond transition timestamp held in this BC (never
/// stored on disk). The canvas formats the elapsed "waiting 2m 14s" string from
/// it locally — no per-second event spam (`agent-awareness-002` AC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskAgentState {
    /// `running | idle | blocked_on_question`.
    pub activity: AgentActivity,
    /// The acting agent label ("orchestrator", "worker", …) for running /
    /// blocked tasks; `None` for idle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_label: Option<String>,
    /// Unix-millisecond transition timestamp the canvas derives the elapsed
    /// duration from. `None` for idle (no transition to time from).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<u64>,
    /// The live question text for `blocked_on_question` (the callout body).
    /// `None` for running / idle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

impl TaskAgentState {
    /// The default `idle` state — no agent, no timestamp, no question.
    pub fn idle() -> Self {
        Self {
            activity: AgentActivity::Idle,
            agent_label: None,
            since: None,
            question: None,
        }
    }

    /// The wire `state` token (`running | idle | blocked_on_question`) — the
    /// same `snake_case` vocabulary [`AgentActivity`] serialises to, materialised
    /// as a `&'static str` for the [`DomainEvent::TaskAgentStateChanged`] payload.
    pub fn state_token(&self) -> &'static str {
        match self.activity {
            AgentActivity::Running => "running",
            AgentActivity::Idle => "idle",
            AgentActivity::BlockedOnQuestion => "blocked_on_question",
        }
    }

    /// Build the bus event the canvas patches a card from. The producer
    /// (`ingest` caller) emits this after folding a signal: the frontend bridge
    /// forwards it under `guppi://event` (ADR-009) and the canvas patches the
    /// matching card's indicator + the panel callout in place.
    pub fn to_event(&self, project_id: i64, bc: &str, task_id: &str) -> DomainEvent {
        DomainEvent::TaskAgentStateChanged {
            project_id,
            bc: bc.to_string(),
            task_id: task_id.to_string(),
            state: self.state_token().to_string(),
            agent_label: self.agent_label.clone(),
            since: self.since,
            question: self.question.clone(),
        }
    }
}

/// A signal folded into the projection. Models the BC README's two-source split
/// behind one type so [`AgentStateProjection::ingest`] is the single fold point
/// and the canvas downstream sees one resulting shape.
///
/// `RunnerRunning` / `RunnerIdle` / `FilesystemBlocked` have no live producer at
/// v1 (ADR-018 wires only the `RunnerBlocked` path through the bus consumer);
/// they are part of the declared, tested contract — runner running/idle
/// transitions and the deferred filesystem-observed fallback (agent-awareness-003)
/// — so adding their producer later touches no consumer. Mirrors the
/// `#[allow(dead_code)]` pattern `events.rs` uses for declared-but-unproduced
/// contract variants.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSignal {
    /// Rich `claude-runner` signal (owned session): the agent began actively
    /// working this task.
    RunnerRunning {
        project_id: i64,
        bc: String,
        task_id: String,
        agent_label: String,
    },
    /// Rich `claude-runner` signal (owned session): the agent is waiting for a
    /// human answer. `question` is the live callout text.
    RunnerBlocked {
        project_id: i64,
        bc: String,
        task_id: String,
        agent_label: String,
        question: String,
    },
    /// Rich `claude-runner` signal (owned session): the agent stopped working
    /// this task (session exited / unblocked / moved on). Transitions the task
    /// back to `idle`.
    RunnerIdle {
        project_id: i64,
        bc: String,
        task_id: String,
    },
    /// Best-effort **filesystem** signal (observed session): a task in `doing/`
    /// carries a `blocked_question:` in its on-disk frontmatter. Lower-fidelity
    /// than the runner path — the disk question is the *fallback* per
    /// `project-registry-005`. No `agent_label` (the filesystem cannot attribute
    /// an acting agent), so the projection labels it `observed`.
    FilesystemBlocked {
        project_id: i64,
        bc: String,
        task_id: String,
        /// The disk `blocked_question:` frontmatter — fallback callout text.
        question: String,
    },
}

impl AgentSignal {
    /// The `(project_id, bc, task_id)` key a signal targets.
    fn key(&self) -> (i64, String, String) {
        match self {
            AgentSignal::RunnerRunning {
                project_id,
                bc,
                task_id,
                ..
            }
            | AgentSignal::RunnerBlocked {
                project_id,
                bc,
                task_id,
                ..
            }
            | AgentSignal::RunnerIdle {
                project_id,
                bc,
                task_id,
            }
            | AgentSignal::FilesystemBlocked {
                project_id,
                bc,
                task_id,
                ..
            } => (*project_id, bc.clone(), task_id.clone()),
        }
    }
}

/// The label the projection stamps on a filesystem-observed (un-attributed)
/// blocked task — the filesystem cannot name the acting agent (BC README:
/// "observed session"). Kept as a constant so the canvas + tests share one
/// vocabulary token.
pub const OBSERVED_AGENT_LABEL: &str = "observed";

/// The in-core per-task agent-state projection (`agent-awareness-002`).
///
/// Keyed by `(project_id, bc, task_id)`. Holds only non-idle states — a task
/// with no live signal is implicitly idle (queried tasks default to
/// [`TaskAgentState::idle`]), keeping the map bounded to what's actually
/// running/blocked.
#[derive(Debug, Default)]
pub struct AgentStateProjection {
    states: HashMap<(i64, String, String), TaskAgentState>,
}

impl AgentStateProjection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold a signal into the projection, stamping the transition timestamp.
    /// Both source fidelities (runner + filesystem) land here — the canvas sees
    /// one resulting [`TaskAgentState`].
    ///
    /// Returns the resulting state so the caller (the bus producer) can emit a
    /// `TaskAgentStateChanged` event from it.
    pub fn ingest(&mut self, signal: AgentSignal) -> TaskAgentState {
        self.ingest_at(signal, now_millis())
    }

    /// `ingest` with an injected timestamp — the testable seam (tests assert
    /// `since` deterministically; production calls `ingest`, which uses the wall
    /// clock).
    pub fn ingest_at(&mut self, signal: AgentSignal, at_millis: u64) -> TaskAgentState {
        let key = signal.key();
        let next = match signal {
            AgentSignal::RunnerRunning { agent_label, .. } => TaskAgentState {
                activity: AgentActivity::Running,
                agent_label: Some(agent_label),
                since: Some(at_millis),
                question: None,
            },
            AgentSignal::RunnerBlocked {
                agent_label,
                question,
                ..
            } => TaskAgentState {
                activity: AgentActivity::BlockedOnQuestion,
                agent_label: Some(agent_label),
                since: Some(at_millis),
                question: Some(question),
            },
            AgentSignal::RunnerIdle { .. } => TaskAgentState::idle(),
            AgentSignal::FilesystemBlocked { question, .. } => TaskAgentState {
                activity: AgentActivity::BlockedOnQuestion,
                agent_label: Some(OBSERVED_AGENT_LABEL.to_string()),
                since: Some(at_millis),
                question: Some(question),
            },
        };

        // Preserve `since` across a no-op re-ingest of the *same* activity so the
        // elapsed timer does not reset on a duplicate signal (e.g. a runner
        // re-emitting "still blocked"). A genuine activity change adopts the new
        // timestamp.
        let next = match self.states.get(&key) {
            Some(prev) if prev.activity == next.activity => TaskAgentState {
                since: prev.since,
                ..next
            },
            _ => next,
        };

        if next.activity == AgentActivity::Idle {
            // Idle is the implicit default — drop it from the map rather than
            // storing it, keeping the projection bounded to live tasks.
            self.states.remove(&key);
            TaskAgentState::idle()
        } else {
            self.states.insert(key, next.clone());
            next
        }
    }

    /// The current state for one task — [`TaskAgentState::idle`] for any task
    /// with no live signal (the implicit default).
    pub fn state_for(&self, project_id: i64, bc: &str, task_id: &str) -> TaskAgentState {
        self.states
            .get(&(project_id, bc.to_string(), task_id.to_string()))
            .cloned()
            .unwrap_or_else(TaskAgentState::idle)
    }

    /// The per-BC roll-up (active / blocked / idling counts) the accordion
    /// header renders. `total_tasks` is the BC's total task count (from the
    /// project-registry snapshot) — `idling` is `total − active − blocked`, so
    /// tasks with no live signal count as idling without the projection storing
    /// an entry for each.
    pub fn rollup(&self, project_id: i64, bc: &str, total_tasks: u32) -> BcRollup {
        let mut active = 0u32;
        let mut blocked = 0u32;
        for ((pid, b, _), state) in &self.states {
            if *pid != project_id || b != bc {
                continue;
            }
            match state.activity {
                AgentActivity::Running => active += 1,
                AgentActivity::BlockedOnQuestion => blocked += 1,
                AgentActivity::Idle => {}
            }
        }
        let idling = total_tasks.saturating_sub(active + blocked);
        BcRollup {
            active,
            blocked,
            idling,
        }
    }
}

/// The per-BC roll-up the accordion header renders: "1 active · 2 blocked ·
/// 1 idling". Derived from the per-task states (`agent-awareness-002`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BcRollup {
    /// Tasks whose agent is `running`.
    pub active: u32,
    /// Tasks whose agent is `blocked_on_question`.
    pub blocked: u32,
    /// Tasks with no live agent signal — `total − active − blocked`.
    pub idling: u32,
}

/// Wall-clock now in Unix milliseconds. Used as the transition timestamp when a
/// signal is ingested in production; tests use [`AgentStateProjection::ingest_at`]
/// with an injected value.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runner_running(task: &str, label: &str) -> AgentSignal {
        AgentSignal::RunnerRunning {
            project_id: 1,
            bc: "agent-awareness".into(),
            task_id: task.into(),
            agent_label: label.into(),
        }
    }

    fn runner_blocked(task: &str, label: &str, q: &str) -> AgentSignal {
        AgentSignal::RunnerBlocked {
            project_id: 1,
            bc: "agent-awareness".into(),
            task_id: task.into(),
            agent_label: label.into(),
            question: q.into(),
        }
    }

    // AC: a per-task projection keyed (project_id, bc, task_id) with the three
    // states + agent label + transition timestamp for running/blocked.
    #[test]
    fn unknown_task_defaults_to_idle() {
        let proj = AgentStateProjection::new();
        let s = proj.state_for(1, "agent-awareness", "agent-awareness-002");
        assert_eq!(s.activity, AgentActivity::Idle);
        assert_eq!(s.agent_label, None);
        assert_eq!(s.since, None);
        assert_eq!(s.question, None);
    }

    #[test]
    fn runner_running_signal_lands_with_label_and_since() {
        let mut proj = AgentStateProjection::new();
        let out = proj.ingest_at(runner_running("t1", "orchestrator"), 1_000);
        assert_eq!(out.activity, AgentActivity::Running);
        assert_eq!(out.agent_label.as_deref(), Some("orchestrator"));
        assert_eq!(out.since, Some(1_000));
        assert_eq!(out.question, None);

        // And it is queryable.
        let s = proj.state_for(1, "agent-awareness", "t1");
        assert_eq!(s, out);
    }

    // AC: blocked-on-question carries the question text the panel callout renders.
    #[test]
    fn runner_blocked_signal_carries_the_question_text() {
        let mut proj = AgentStateProjection::new();
        let out = proj.ingest_at(
            runner_blocked("t1", "worker", "Dock the panel left or right?"),
            2_000,
        );
        assert_eq!(out.activity, AgentActivity::BlockedOnQuestion);
        assert_eq!(out.agent_label.as_deref(), Some("worker"));
        assert_eq!(out.since, Some(2_000));
        assert_eq!(out.question.as_deref(), Some("Dock the panel left or right?"));
    }

    #[test]
    fn runner_idle_signal_clears_a_live_task_back_to_idle() {
        let mut proj = AgentStateProjection::new();
        proj.ingest_at(runner_running("t1", "orchestrator"), 1_000);
        let out = proj.ingest_at(
            AgentSignal::RunnerIdle {
                project_id: 1,
                bc: "agent-awareness".into(),
                task_id: "t1".into(),
            },
            3_000,
        );
        assert_eq!(out.activity, AgentActivity::Idle);
        assert_eq!(proj.state_for(1, "agent-awareness", "t1").activity, AgentActivity::Idle);
    }

    // AC: elapsed-duration is derivable from `since`; a duplicate same-activity
    // signal must NOT reset the timer (no per-second spam, stable elapsed base).
    #[test]
    fn re_ingesting_same_activity_preserves_the_since_timestamp() {
        let mut proj = AgentStateProjection::new();
        proj.ingest_at(runner_blocked("t1", "worker", "Q?"), 5_000);
        // The runner re-emits "still blocked" later — `since` must NOT jump.
        let out = proj.ingest_at(runner_blocked("t1", "worker", "Q?"), 9_999);
        assert_eq!(out.since, Some(5_000), "elapsed base must be stable across a duplicate signal");
    }

    #[test]
    fn a_genuine_activity_change_adopts_the_new_timestamp() {
        let mut proj = AgentStateProjection::new();
        proj.ingest_at(runner_running("t1", "orchestrator"), 5_000);
        let out = proj.ingest_at(runner_blocked("t1", "orchestrator", "Q?"), 8_000);
        assert_eq!(out.activity, AgentActivity::BlockedOnQuestion);
        assert_eq!(out.since, Some(8_000), "a state transition adopts the new timestamp");
    }

    // AC: the runner-vs-filesystem source split is preserved — both an owned
    // (runner) blocked signal and a filesystem-observed blocked signal land in
    // the unified per-task model. (v1 fidelity: runner is primary/live,
    // filesystem is fallback — ADR-018.)
    #[test]
    fn filesystem_blocked_signal_lands_in_the_same_unified_model() {
        let mut proj = AgentStateProjection::new();
        let out = proj.ingest_at(
            AgentSignal::FilesystemBlocked {
                project_id: 1,
                bc: "agent-awareness".into(),
                task_id: "observed-1".into(),
                question: "On-disk fallback question?".into(),
            },
            4_000,
        );
        assert_eq!(out.activity, AgentActivity::BlockedOnQuestion);
        // Filesystem cannot attribute an acting agent — labelled `observed`.
        assert_eq!(out.agent_label.as_deref(), Some(OBSERVED_AGENT_LABEL));
        assert_eq!(out.question.as_deref(), Some("On-disk fallback question?"));

        // Both sources coexist in one projection; the canvas reads one shape.
        proj.ingest_at(runner_blocked("owned-1", "worker", "Live question?"), 4_500);
        let owned = proj.state_for(1, "agent-awareness", "owned-1");
        let observed = proj.state_for(1, "agent-awareness", "observed-1");
        assert_eq!(owned.activity, AgentActivity::BlockedOnQuestion);
        assert_eq!(observed.activity, AgentActivity::BlockedOnQuestion);
        assert_eq!(owned.question.as_deref(), Some("Live question?"));
        assert_eq!(observed.question.as_deref(), Some("On-disk fallback question?"));
    }

    // AC: a per-BC roll-up (active / blocked / idling) derived from per-task
    // states, for the accordion header.
    #[test]
    fn rollup_counts_active_blocked_and_idling() {
        let mut proj = AgentStateProjection::new();
        proj.ingest_at(runner_running("t1", "orchestrator"), 1_000);
        proj.ingest_at(runner_blocked("t2", "worker", "Q1?"), 1_000);
        proj.ingest_at(runner_blocked("t3", "worker", "Q2?"), 1_000);
        // total_tasks = 4 → one task (t4) has no live signal → idling.
        let r = proj.rollup(1, "agent-awareness", 4);
        assert_eq!(r.active, 1);
        assert_eq!(r.blocked, 2);
        assert_eq!(r.idling, 1, "the BC's reference header reads 1 active · 2 blocked · 1 idling");
    }

    #[test]
    fn rollup_is_scoped_to_one_project_and_bc() {
        let mut proj = AgentStateProjection::new();
        proj.ingest_at(runner_running("t1", "orchestrator"), 1_000);
        // A running task in a DIFFERENT bc must not bleed into this bc's rollup.
        proj.ingest_at(
            AgentSignal::RunnerRunning {
                project_id: 1,
                bc: "canvas".into(),
                task_id: "c1".into(),
                agent_label: "worker".into(),
            },
            1_000,
        );
        // And a different project entirely.
        proj.ingest_at(
            AgentSignal::RunnerBlocked {
                project_id: 2,
                bc: "agent-awareness".into(),
                task_id: "p2t1".into(),
                agent_label: "worker".into(),
                question: "Other project?".into(),
            },
            1_000,
        );
        let r = proj.rollup(1, "agent-awareness", 3);
        assert_eq!(r.active, 1);
        assert_eq!(r.blocked, 0);
        assert_eq!(r.idling, 2);
    }

    // AC: the canvas can read per-task state via a documented event variant.
    // The projection's output maps to `DomainEvent::TaskAgentStateChanged` with
    // the `snake_case` `state` token + optional label/since/question — the
    // contract canvas-020 consumes.
    #[test]
    fn blocked_state_serialises_to_the_documented_event_shape() {
        let mut proj = AgentStateProjection::new();
        let out = proj.ingest_at(runner_blocked("t1", "orchestrator", "Left or right?"), 7_000);
        let event = out.to_event(1, "agent-awareness", "t1");
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["kind"], "task_agent_state_changed");
        assert_eq!(json["project_id"], 1);
        assert_eq!(json["bc"], "agent-awareness");
        assert_eq!(json["task_id"], "t1");
        assert_eq!(json["state"], "blocked_on_question");
        assert_eq!(json["agent_label"], "orchestrator");
        assert_eq!(json["since"], 7_000);
        assert_eq!(json["question"], "Left or right?");
    }

    #[test]
    fn idle_state_omits_label_since_and_question_on_the_wire() {
        let s = TaskAgentState::idle();
        let event = s.to_event(1, "agent-awareness", "t9");
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["state"], "idle");
        assert!(json.get("agent_label").is_none(), "idle omits agent_label");
        assert!(json.get("since").is_none(), "idle omits since");
        assert!(json.get("question").is_none(), "idle omits question");
    }

    #[test]
    fn rollup_idling_never_underflows() {
        // If live signals somehow exceed total_tasks (a transient race between
        // the snapshot count and the live projection), idling clamps at 0 rather
        // than underflowing.
        let mut proj = AgentStateProjection::new();
        proj.ingest_at(runner_running("t1", "orchestrator"), 1_000);
        proj.ingest_at(runner_running("t2", "orchestrator"), 1_000);
        let r = proj.rollup(1, "agent-awareness", 1);
        assert_eq!(r.idling, 0, "idling must clamp at 0, never underflow");
    }
}
