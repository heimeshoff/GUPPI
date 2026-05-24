// guppi-palette.jsx — keyboard-driven command palette.

const ViewCommandPalette = () => {
  const items = [
    { kind: 'cmd',     icon: '⌘', label: 'focus project',  arg: 'image-gallery', kbd: '⌘1', voice: '"focus image-gallery"' },
    { kind: 'cmd',     icon: '⌘', label: 'open BC',         arg: 'image-gallery / media',     kbd: null, voice: '"open media"' },
    { kind: 'task',    icon: '▶', label: 'refine task',     arg: '104  ·  thumbnail strategy', kbd: null, voice: '"refine task 104"' },
    { kind: 'task',    icon: '◆', label: 'answer blocked',  arg: 'payment-rails / ledger',     kbd: '⌘A', voice: '"answer ledger"' },
    { kind: 'cmd',     icon: '⌘', label: 'new project',     arg: 'folder + git init + brainstorm', kbd: '⌘N', voice: '"new project"' },
    { kind: 'cmd',     icon: '⌘', label: 'mute Bob',        arg: 'voice off until ⌘M',         kbd: '⌘M', voice: '"mute"' },
    { kind: 'session', icon: '▶', label: 'show terminal',   arg: 'image-gallery / media',      kbd: '⌘T', voice: '"show terminal"' },
  ];

  return (
    <CanvasGrid w={1000} h={680}>
      <div style={{ position: 'absolute', left: 24, top: 14, color: 'var(--g-fg-4)', fontSize: 10, fontFamily: 'var(--g-font-mono)' }}>
        command palette · ⌘K · same vocabulary as voice
      </div>

      {/* Faded canvas context behind */}
      <div style={{ position: 'absolute', inset: 0, opacity: 0.5 }}>
        <MiniProjectTile x={90}  y={80}  w={260} h={150} scale={0.92}
          name="image-gallery"
          counts={{ running: 1, blocked: 0, missing: 0, idle: 3 }}
          bcs={[{ name: 'media', status: 'running' }, { name: 'albums', status: 'idle' }, { name: 'sharing', status: 'idle' }, { name: 'thumbnails', status: 'idle' }]} dim />
        <MiniProjectTile x={400} y={80}  w={260} h={150} scale={0.92}
          name="payment-rails"
          counts={{ running: 1, blocked: 1, missing: 0, idle: 2 }}
          bcs={[{ name: 'ledger', status: 'blocked' }, { name: 'webhook', status: 'running' }, { name: 'fx', status: 'idle' }]} dim />
        <MiniProjectTile x={710} y={120} w={240} h={140} scale={0.85}
          name="auth-service"
          counts={{ running: 2, blocked: 0, missing: 0, idle: 1 }}
          bcs={[{ name: 'identity', status: 'running' }, { name: 'sessions', status: 'running' }, { name: 'audit', status: 'idle' }]} dim />
      </div>

      {/* Modal scrim */}
      <div style={{ position: 'absolute', inset: 0, background: 'var(--g-overlay)' }} />

      {/* Palette */}
      <div style={{
        position: 'absolute', left: '50%', top: 90, transform: 'translateX(-50%)',
        width: 640,
        background: 'var(--g-surface-1)',
        border: '1px solid var(--g-hairline-strong)',
        borderRadius: 12,
        overflow: 'hidden',
      }}>
        {/* Input */}
        <div style={{
          display: 'flex', alignItems: 'center', gap: 12,
          padding: '14px 18px',
          borderBottom: '1px solid var(--g-hairline)',
        }}>
          <svg width="14" height="14" viewBox="0 0 14 14"><circle cx="6" cy="6" r="4.25" fill="none" stroke="var(--g-fg-3)" strokeWidth="1.25" /><path d="M9 9 L12.5 12.5" stroke="var(--g-fg-3)" strokeWidth="1.25" strokeLinecap="round" /></svg>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 0, flex: '1 1 auto', fontSize: 14, fontFamily: 'var(--g-font-sans)' }}>
            <span style={{ color: 'var(--g-fg-1)' }}>refine task</span>
            <span style={{ width: 1, height: 16, background: 'var(--g-fg-1)', marginLeft: 1, animation: 'g-blink 1.1s steps(2) infinite', transform: 'translateY(2px)' }} />
          </div>
          <span className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)' }}>7 results</span>
          <span className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', padding: '2px 6px', border: '1px solid var(--g-hairline-strong)', borderRadius: 4 }}>esc</span>
        </div>

        {/* Results */}
        <div style={{ padding: 6, maxHeight: 420, overflow: 'auto' }}>
          {items.map((it, i) => {
            const active = i === 2;
            return (
              <div key={i} style={{
                display: 'flex', alignItems: 'center', gap: 12,
                padding: '8px 12px', borderRadius: 6, cursor: 'pointer',
                background: active ? 'var(--g-surface-4)' : 'transparent',
                position: 'relative',
              }}>
                {active && <span style={{ position: 'absolute', left: 0, top: 8, bottom: 8, width: 2, background: 'var(--g-periwinkle)', borderRadius: 2 }} />}
                <span style={{
                  width: 22, height: 22, display: 'grid', placeItems: 'center',
                  fontFamily: 'var(--g-font-mono)', fontSize: 11,
                  color: it.kind === 'task' ? 'var(--g-status-running)' : 'var(--g-fg-3)',
                  background: 'var(--g-surface-2)',
                  borderRadius: 4,
                }}>{it.icon}</span>
                <div style={{ flex: '1 1 auto', minWidth: 0, display: 'flex', alignItems: 'baseline', gap: 12 }}>
                  <span style={{ fontSize: 13, color: 'var(--g-fg-1)' }}>
                    {/* Highlight matched substring */}
                    {it.label.split('').map((c, j) => {
                      const match = 'refinetask'.includes(c.toLowerCase());
                      return <span key={j} style={{ color: match && i === 2 ? 'var(--g-fg-1)' : (i === 2 ? 'var(--g-fg-2)' : 'var(--g-fg-1)') }}>{c}</span>;
                    })}
                  </span>
                  <span style={{ fontSize: 12, color: 'var(--g-fg-3)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {it.arg}
                  </span>
                </div>
                <span className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', minWidth: 140, textAlign: 'right' }}>
                  {it.voice}
                </span>
                {it.kbd && (
                  <span className="mono" style={{
                    fontSize: 10, color: 'var(--g-fg-3)',
                    padding: '2px 6px', border: '1px solid var(--g-hairline-strong)', borderRadius: 4,
                  }}>{it.kbd}</span>
                )}
              </div>
            );
          })}
        </div>

        {/* Footer */}
        <div style={{
          display: 'flex', alignItems: 'center', gap: 16,
          padding: '10px 16px',
          borderTop: '1px solid var(--g-hairline)',
          fontFamily: 'var(--g-font-mono)', fontSize: 10, color: 'var(--g-fg-3)',
        }}>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <Kbd>↑</Kbd><Kbd>↓</Kbd> navigate
          </span>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <Kbd>↩</Kbd> run
          </span>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <Kbd>⌘</Kbd><Kbd>↩</Kbd> run + stay
          </span>
          <span style={{ marginLeft: 'auto', color: 'var(--g-fg-4)' }}>
            same vocabulary as voice — every command can be spoken
          </span>
        </div>
      </div>

      <VoiceIndicator state="idle" />

      <style>{`@keyframes g-blink { 0%,49%{opacity:1} 50%,100%{opacity:0} }`}</style>
    </CanvasGrid>
  );
};

const Kbd = ({ children }) => (
  <span className="mono" style={{
    fontSize: 9.5, color: 'var(--g-fg-2)',
    padding: '1px 5px', border: '1px solid var(--g-hairline-strong)', borderRadius: 3,
  }}>{children}</span>
);

Object.assign(window, { ViewCommandPalette, Kbd });
