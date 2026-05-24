// guppi-panels.jsx — project detail (markdown reader) + terminal panel.

// ───────── Project detail panel ─────────
// Slides in from the right when a project is focused. Renders markdown
// designed for long-form reading on a dark background.
const ViewDetailPanel = () => (
  <CanvasGrid w={1000} h={680}>
    <div style={{ position: 'absolute', left: 24, top: 14, color: 'var(--g-fg-4)', fontSize: 10, fontFamily: 'var(--g-font-mono)' }}>
      project detail panel · slides in from right · markdown reader
    </div>

    {/* Faded canvas behind, so it feels like a panel ON the canvas */}
    <ProjectFrame
      name="payment-rails" title="payment-rails"
      x={24} y={50} w={360} h={300}
      counts={{ running: 1, blocked: 1, missing: 0, idle: 2 }}
      taskLine="8 tasks · 1 active · 1 blocked"
      dim
    >
      <BCBubble name="ledger"   status="blocked" blocked={1} x={20}  y={28}  w={146} h={56} />
      <BCBubble name="webhook"  status="running" running={1} x={184} y={28}  w={146} h={56} />
      <BCBubble name="fx"       status="idle"    idle={2}    x={20}  y={104} w={146} h={56} />
      <BCBubble name="reports"  status="idle"    idle={1}    x={184} y={104} w={146} h={56} />
    </ProjectFrame>

    {/* The panel */}
    <div style={{
      position: 'absolute', right: 24, top: 28, bottom: 28, width: 520,
      background: 'var(--g-surface-3)',
      border: '1px solid var(--g-hairline-strong)',
      borderRadius: 'var(--g-r-panel)',
      display: 'flex', flexDirection: 'column',
      overflow: 'hidden',
    }}>
      {/* Panel header */}
      <div style={{
        height: 44, display: 'flex', alignItems: 'center', gap: 12,
        padding: '0 18px', borderBottom: '1px solid var(--g-hairline)',
        flex: '0 0 44px',
      }}>
        <span style={{ fontSize: 12, color: 'var(--g-fg-3)' }}>payment-rails</span>
        <span style={{ color: 'var(--g-fg-4)' }}>/</span>
        <span style={{ fontSize: 12, color: 'var(--g-fg-1)' }}>vision.md</span>
        <span className="mono" style={{ marginLeft: 'auto', fontSize: 10, color: 'var(--g-fg-4)' }}>1,284 words · 6 min</span>
        <button style={{
          background: 'transparent', border: 'none', color: 'var(--g-fg-3)',
          fontSize: 14, cursor: 'pointer', padding: 4, marginLeft: 4,
        }}>×</button>
      </div>

      {/* Tabs (file picker for the project's docs) */}
      <div style={{
        display: 'flex', gap: 0, padding: '0 18px',
        borderBottom: '1px solid var(--g-hairline)',
        flex: '0 0 36px', alignItems: 'stretch',
      }}>
        {[
          { name: 'vision.md', active: true },
          { name: 'ADRs', active: false },
          { name: 'research', active: false },
          { name: 'BC READMEs', active: false },
        ].map(t => (
          <div key={t.name} style={{
            display: 'flex', alignItems: 'center', padding: '0 14px',
            fontSize: 11, color: t.active ? 'var(--g-fg-1)' : 'var(--g-fg-3)',
            borderBottom: t.active ? '1px solid var(--g-periwinkle)' : '1px solid transparent',
            marginBottom: -1, cursor: 'pointer',
          }}>{t.name}</div>
        ))}
      </div>

      {/* Markdown body */}
      <div style={{
        flex: '1 1 auto', overflow: 'auto',
        padding: '32px 48px 48px',
        fontFamily: 'var(--g-font-sans)',
        fontSize: 14, lineHeight: 1.65,
        color: 'var(--g-fg-1)',
        maxWidth: 620,
        textWrap: 'pretty',
      }}>
        <div style={{ fontFamily: 'var(--g-font-mono)', fontSize: 10, color: 'var(--g-fg-4)', letterSpacing: '0.08em', textTransform: 'uppercase', marginBottom: 16 }}>
          vision
        </div>
        <h1 style={{ fontSize: 22, fontWeight: 600, margin: '0 0 6px 0', letterSpacing: '-0.015em', color: 'var(--g-fg-1)' }}>
          A ledger that never lies, even when the world reorders itself.
        </h1>
        <p style={{ color: 'var(--g-fg-3)', fontSize: 13, margin: '0 0 28px 0' }}>
          Drafted Apr 12, 2026 · last revised today by orchestrator
        </p>

        <p style={{ margin: '0 0 16px 0', color: 'var(--g-fg-2)' }}>
          The payment-rails service exists for one reason: at any point in time, the sum of money
          across all accounts must equal the sum of money in motion plus the sum at rest.
          This invariant survives clock skew, retries, network partitions, and the customer-supplier
          relationships with <span style={{ color: 'var(--g-fg-1)' }}>image-gallery</span> and
          <span style={{ color: 'var(--g-fg-1)' }}> auth-service</span>.
        </p>

        <h2 style={{ fontSize: 15, fontWeight: 600, margin: '32px 0 8px 0', color: 'var(--g-fg-1)' }}>
          Bounded contexts
        </h2>
        <p style={{ margin: '0 0 12px 0', color: 'var(--g-fg-2)' }}>
          Four contexts, each with one job and one neighbor it talks to politely:
        </p>
        <ul style={{ margin: '0 0 16px 0', padding: '0 0 0 20px', color: 'var(--g-fg-2)' }}>
          <li style={{ margin: '0 0 6px 0' }}><span style={{ color: 'var(--g-fg-1)' }}>ledger</span> — the double-entry core. Never speaks of dollars, only of entries.</li>
          <li style={{ margin: '0 0 6px 0' }}><span style={{ color: 'var(--g-fg-1)' }}>webhook</span> — the supplier; converts external events into ledger commands.</li>
          <li style={{ margin: '0 0 6px 0' }}><span style={{ color: 'var(--g-fg-1)' }}>fx</span> — translates currencies at the boundary, never inside.</li>
          <li style={{ margin: '0 0 6px 0' }}><span style={{ color: 'var(--g-fg-1)' }}>reports</span> — read model. Eventually consistent. Allowed to be wrong for seconds.</li>
        </ul>

        <h2 style={{ fontSize: 15, fontWeight: 600, margin: '32px 0 8px 0', color: 'var(--g-fg-1)' }}>
          Non-goals
        </h2>
        <p style={{ margin: '0 0 12px 0', color: 'var(--g-fg-2)' }}>
          payment-rails is not a wallet, an exchange, or a fraud system. Anything that smells like
          policy lives outside the context; the ledger only records what the policy decides.
        </p>

        <blockquote style={{
          margin: '24px 0', padding: '12px 18px',
          borderLeft: '2px solid var(--g-periwinkle-soft)',
          background: 'var(--g-surface-1)',
          color: 'var(--g-fg-2)',
          fontSize: 13, lineHeight: 1.6,
          borderRadius: '0 4px 4px 0',
        }}>
          A correct number on the wrong day is wrong. A correct number ten seconds late is correct.
        </blockquote>

        <h2 style={{ fontSize: 15, fontWeight: 600, margin: '32px 0 8px 0', color: 'var(--g-fg-1)' }}>
          The invariant, in code
        </h2>
        <pre style={{
          margin: '12px 0 20px',
          padding: '14px 16px',
          background: 'var(--g-surface-1)',
          border: '1px solid var(--g-hairline)',
          borderRadius: 6,
          fontFamily: 'var(--g-font-mono)',
          fontSize: 12,
          lineHeight: 1.55,
          color: 'var(--g-fg-1)',
          overflow: 'auto',
        }}>
{`function assert_invariant(ledger: Ledger) {
  const sum_of_entries = ledger.entries
    .reduce((a, e) => a + e.amount_minor_units, 0n);
  if (sum_of_entries !== 0n) {
    throw new LedgerCorruption(sum_of_entries);
  }
}`}
        </pre>

        <p style={{ margin: '0 0 16px 0', color: 'var(--g-fg-2)' }}>
          The function is called on every commit. It has never returned non-zero in production.
          The day it does is the day we stop the world.
        </p>
      </div>
    </div>

    <VoiceIndicator state="idle" />
  </CanvasGrid>
);

// ───────── Terminal panel ─────────
// Emulated Claude session: orchestrator + 2 sub-agents, with one of them
// blocked on a question. Looks like a terminal, reads like a doc.
const ViewTerminalPanel = () => (
  <CanvasGrid w={1000} h={680}>
    <div style={{ position: 'absolute', left: 24, top: 14, color: 'var(--g-fg-4)', fontSize: 10, fontFamily: 'var(--g-font-mono)' }}>
      terminal panel · orchestrator + sub-agents · readable, not raw
    </div>

    <div style={{
      position: 'absolute', left: 24, top: 40, right: 24, bottom: 24,
      background: 'var(--g-surface-3)',
      border: '1px solid var(--g-hairline-strong)',
      borderRadius: 'var(--g-r-panel)',
      display: 'flex', flexDirection: 'column',
      overflow: 'hidden',
    }}>
      {/* Header */}
      <div style={{
        height: 40, flex: '0 0 40px',
        display: 'flex', alignItems: 'center', gap: 12,
        padding: '0 16px', borderBottom: '1px solid var(--g-hairline)',
        fontFamily: 'var(--g-font-mono)', fontSize: 11, color: 'var(--g-fg-2)',
      }}>
        <span style={{ display: 'inline-flex', gap: 5 }}>
          <span style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--g-hairline-strong)' }} />
          <span style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--g-hairline-strong)' }} />
          <span style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--g-hairline-strong)' }} />
        </span>
        <span style={{ color: 'var(--g-fg-3)' }}>image-gallery</span>
        <span style={{ color: 'var(--g-fg-4)' }}>·</span>
        <span style={{ color: 'var(--g-fg-1)' }}>media</span>
        <span style={{ color: 'var(--g-fg-4)' }}>·</span>
        <span style={{ color: 'var(--g-fg-3)' }}>session 0x4f2c</span>
        <span style={{ marginLeft: 'auto', display: 'inline-flex', gap: 16, color: 'var(--g-fg-3)' }}>
          <span><StatusGlyph status="running" size={9} /> 2</span>
          <span><StatusGlyph status="blocked" size={9} /> 1</span>
          <span>elapsed 4m 18s</span>
        </span>
      </div>

      {/* Body */}
      <div style={{ flex: '1 1 auto', overflow: 'auto', padding: '14px 18px 18px', fontFamily: 'var(--g-font-mono)', fontSize: 12, lineHeight: 1.55, color: 'var(--g-fg-2)' }}>
        {/* Orchestrator turn */}
        <TermLine agent="orchestrator" agentColor="var(--g-periwinkle)" time="14:02:11">
          Goal: implement <span style={{ color: 'var(--g-fg-1)' }}>media</span> uploads with thumbnail generation.
          Splitting into two parallel sub-agents.
        </TermLine>
        <TermLine agent="orchestrator" agentColor="var(--g-periwinkle)" time="14:02:14">
          Spawning <span style={{ color: 'var(--g-status-running)' }}>uploader</span> and <span style={{ color: 'var(--g-status-running)' }}>thumb-gen</span>.
        </TermLine>

        {/* Collapsible sub-agent thread */}
        <SubAgent name="uploader" status="running" color="var(--g-teal)" elapsed="3m 02s">
          <TermLine agent="uploader" agentColor="var(--g-teal)" time="14:02:18" indent>
            Reading <span style={{ color: 'var(--g-fg-1)' }}>src/media/upload.ts</span>… 142 lines.
          </TermLine>
          <TermLine agent="uploader" agentColor="var(--g-teal)" time="14:02:24" indent>
            Plan: introduce <code style={codeStyle}>StorageBackend</code> port, default to S3, multipart for &gt;5MB.
          </TermLine>
          <TermLine agent="uploader" agentColor="var(--g-teal)" time="14:02:48" indent>
            <span style={{ color: 'var(--g-fg-3)' }}>edit</span> src/media/upload.ts <span style={{ color: 'var(--g-status-running)' }}>+82 −14</span>
          </TermLine>
          <TermLine agent="uploader" agentColor="var(--g-teal)" time="14:03:05" indent>
            <span style={{ color: 'var(--g-fg-3)' }}>test</span> media/upload.test.ts <span style={{ color: 'var(--g-status-running)' }}>12 passed</span>
          </TermLine>
          <TermLine agent="uploader" agentColor="var(--g-teal)" time="14:05:14" indent muted>
            …drafting integration test for multipart resume.
          </TermLine>
        </SubAgent>

        {/* Sub-agent that's blocked */}
        <SubAgent name="thumb-gen" status="blocked" color="var(--g-status-blocked)" elapsed="2m 14s" highlighted>
          <TermLine agent="thumb-gen" agentColor="var(--g-status-blocked)" time="14:02:18" indent>
            Reading <span style={{ color: 'var(--g-fg-1)' }}>src/media/thumbnails.ts</span>… not found.
          </TermLine>
          <TermLine agent="thumb-gen" agentColor="var(--g-status-blocked)" time="14:02:22" indent>
            Inspecting BC README for thumbnail strategy.
          </TermLine>
          <TermLine agent="thumb-gen" agentColor="var(--g-status-blocked)" time="14:02:31" indent>
            Two candidates: sharp (native, fast, +bin) or jimp (pure-JS, portable).
          </TermLine>

          {/* The "agent is asking you something" affordance */}
          <div style={{
            margin: '10px 0 6px 24px',
            padding: '12px 14px',
            background: 'var(--g-status-blocked-glow)',
            border: '1px solid var(--g-status-blocked)',
            borderLeft: '3px solid var(--g-status-blocked)',
            borderRadius: 6,
            fontFamily: 'var(--g-font-sans)',
            fontSize: 12,
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <StatusBadge status="blocked" size={14} />
              <span className="mono" style={{ fontSize: 10, color: 'var(--g-status-blocked)', letterSpacing: '0.04em', textTransform: 'uppercase' }}>thumb-gen needs an answer</span>
              <span className="mono" style={{ marginLeft: 'auto', fontSize: 10, color: 'var(--g-fg-4)' }}>waiting 1m 47s</span>
            </div>
            <div style={{ color: 'var(--g-fg-1)', lineHeight: 1.5, marginBottom: 10 }}>
              Use <code style={codeStyle}>sharp</code> (native, ~4× faster, adds a 12 MB native binary)
              or <code style={codeStyle}>jimp</code> (pure-JS, portable, slower)? The project doesn't
              currently ship native deps.
            </div>
            <div style={{ display: 'flex', gap: 6 }}>
              <button style={btnGhost}>sharp</button>
              <button style={btnGhost}>jimp</button>
              <button style={btnGhost}>let me think</button>
              <button style={{ ...btnGhost, marginLeft: 'auto', color: 'var(--g-fg-3)' }}>type ↵</button>
            </div>
          </div>
        </SubAgent>
      </div>

      {/* Input bar */}
      <div style={{
        flex: '0 0 44px',
        borderTop: '1px solid var(--g-hairline)',
        padding: '0 18px',
        display: 'flex', alignItems: 'center', gap: 10,
        fontFamily: 'var(--g-font-mono)', fontSize: 12, color: 'var(--g-fg-2)',
      }}>
        <span style={{ color: 'var(--g-periwinkle)' }}>›</span>
        <span style={{ color: 'var(--g-fg-3)' }}>answer thumb-gen, or speak to orchestrator</span>
        <span style={{ marginLeft: 'auto', color: 'var(--g-fg-4)', fontSize: 10 }}>⌘↩ to send · ⌘⇧↵ for newline</span>
      </div>
    </div>

    <VoiceIndicator state="idle" />
  </CanvasGrid>
);

const codeStyle = {
  fontFamily: 'var(--g-font-mono)',
  background: 'var(--g-surface-1)',
  border: '1px solid var(--g-hairline)',
  borderRadius: 3, padding: '0 4px',
  fontSize: 11.5,
  color: 'var(--g-fg-1)',
};

const TermLine = ({ agent, agentColor, time, children, indent = false, muted = false }) => (
  <div style={{
    display: 'flex', gap: 12, marginBottom: 4,
    paddingLeft: indent ? 16 : 0,
    color: muted ? 'var(--g-fg-3)' : 'var(--g-fg-2)',
  }}>
    <span className="mono" style={{ color: 'var(--g-fg-4)', fontSize: 10.5, flex: '0 0 50px' }}>{time}</span>
    <span style={{ color: agentColor, flex: '0 0 88px' }}>{agent}</span>
    <span style={{ flex: '1 1 auto' }}>{children}</span>
  </div>
);

const SubAgent = ({ name, status, color, elapsed, highlighted = false, children }) => (
  <div style={{
    margin: '10px 0',
    borderLeft: `2px solid ${highlighted ? 'var(--g-status-blocked)' : 'var(--g-hairline-strong)'}`,
    paddingLeft: 10,
  }}>
    <div style={{
      display: 'flex', alignItems: 'center', gap: 10,
      marginBottom: 4,
      fontFamily: 'var(--g-font-mono)', fontSize: 11,
    }}>
      <span style={{ color: 'var(--g-fg-3)' }}>┌</span>
      <StatusGlyph status={status} size={9} />
      <span style={{ color }}>{name}</span>
      <span style={{ color: 'var(--g-fg-4)' }}>· elapsed {elapsed}</span>
      <span style={{ color: 'var(--g-fg-4)', marginLeft: 'auto' }}>▾</span>
    </div>
    {children}
  </div>
);

Object.assign(window, { ViewDetailPanel, ViewTerminalPanel, TermLine, SubAgent });
