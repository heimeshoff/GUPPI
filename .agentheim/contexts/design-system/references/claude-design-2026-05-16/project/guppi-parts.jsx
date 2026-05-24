// guppi-parts.jsx — shared GUPPI atoms.
// Status badges, BC bubbles, project frames, DDD edges, voice indicator,
// rationale plates. Everything an artboard composes from.

// ───────── Status atoms ─────────
// Glyph is geometric (colorblind-safe) — color is redundant, not load-bearing.
const StatusGlyph = ({ status, size = 10, color }) => {
  const stroke = color || `var(--g-status-${status})`;
  const fill   = color || `var(--g-status-${status})`;
  const half = size / 2;
  switch (status) {
    case 'idle':    // hollow circle
      return (
        <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ display: 'block' }}>
          <circle cx={half} cy={half} r={half - 1} fill="none" stroke={stroke} strokeWidth="1.25" />
        </svg>
      );
    case 'running': // play triangle
      return (
        <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ display: 'block' }}>
          <path d={`M ${size * 0.22} ${size * 0.14} L ${size * 0.86} ${half} L ${size * 0.22} ${size * 0.86} Z`} fill={fill} />
        </svg>
      );
    case 'blocked': // diamond
      return (
        <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ display: 'block' }}>
          <path d={`M ${half} 0.6 L ${size - 0.6} ${half} L ${half} ${size - 0.6} L 0.6 ${half} Z`} fill={fill} />
        </svg>
      );
    case 'missing': // ✕
      return (
        <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ display: 'block' }}>
          <path d={`M 1.4 1.4 L ${size - 1.4} ${size - 1.4} M ${size - 1.4} 1.4 L 1.4 ${size - 1.4}`} stroke={stroke} strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      );
    default: return null;
  }
};

// Small filled chip in the BC's top-right corner.
const StatusBadge = ({ status, pulse = false, size = 14 }) => {
  const bg    = `var(--g-status-${status})`;
  return (
    <div
      style={{
        width: size, height: size,
        borderRadius: size / 2,
        background: `color-mix(in oklab, ${bg} 22%, transparent)`,
        border: `1px solid color-mix(in oklab, ${bg} 60%, transparent)`,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        animation: pulse && status === 'running'
          ? `g-pulse-running var(--g-pulse) var(--g-ease) infinite` : 'none',
      }}
    >
      <StatusGlyph status={status} size={size * 0.55} />
    </div>
  );
};

// Mini badge with count alongside, used in project frame header.
const StatusMini = ({ status, count }) => (
  <span
    style={{
      display: 'inline-flex', alignItems: 'center', gap: 4,
      fontFamily: 'var(--g-font-mono)', fontSize: 10,
      color: count > 0 ? `var(--g-status-${status})` : 'var(--g-fg-4)',
      fontFeatureSettings: '"tnum","zero"',
    }}
  >
    <StatusGlyph status={status} size={8} color={count > 0 ? undefined : 'var(--g-fg-4)'} />
    {String(count).padStart(1, '0')}
  </span>
);

// ───────── BC bubble ─────────
const BCBubble = ({ name, status = 'idle', running = 0, blocked = 0, missing = 0, idle = 0, x, y, w = 168, h = 48, focused = false }) => {
  return (
    <div
      data-bc={name}
      style={{
        position: 'absolute', left: x, top: y, width: w, height: h,
        background: 'var(--g-surface-2)',
        border: `1px solid ${focused ? 'var(--g-focus-ring)' : 'var(--g-teal-soft)'}`,
        borderRadius: 'var(--g-r-bc)',
        padding: '8px 10px 8px 10px',
        display: 'flex', flexDirection: 'column', justifyContent: 'space-between',
        animation: status === 'running' ? `g-pulse-running var(--g-pulse) var(--g-ease) infinite` : 'none',
      }}
    >
      {/* Top row: name + status badge */}
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 8 }}>
        <span style={{ fontSize: 12, color: 'var(--g-fg-1)', letterSpacing: '-0.005em' }}>{name}</span>
        <StatusBadge status={status} pulse={status === 'running'} />
      </div>
      {/* Bottom row: count pill, mono */}
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10, fontFamily: 'var(--g-font-mono)', fontSize: 10, color: 'var(--g-fg-3)', fontFeatureSettings: '"tnum"' }}>
        {running > 0 && <span style={{ color: 'var(--g-status-running)' }}>▶{running}</span>}
        {blocked > 0 && <span style={{ color: 'var(--g-status-blocked)' }}>◆{blocked}</span>}
        {missing > 0 && <span style={{ color: 'var(--g-status-missing)' }}>✕{missing}</span>}
        {idle > 0 && <span>○{idle}</span>}
        {running + blocked + missing + idle === 0 && <span style={{ color: 'var(--g-fg-4)' }}>—</span>}
      </div>
    </div>
  );
};

// ───────── DDD edges ─────────
// Single neutral color (#3a3b46). Geometry distinguishes the type.
//   customer-supplier:    arrow head
//   shared-kernel:        plain line, no arrowhead
//   anti-corruption:      arrow + midpoint triangle notch (one-way valve)
//   conformist:           lighter arrow
const DDDEdge = ({ from, to, kind = 'customer-supplier', svgW = 800, svgH = 600 }) => {
  // from/to are { x, y } in svg-space (artboard local).
  const dx = to.x - from.x, dy = to.y - from.y;
  const len = Math.hypot(dx, dy) || 1;
  const ux = dx / len, uy = dy / len;
  const mid = { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 };

  // Trim ends so the line doesn't sit ON the bubble border.
  const trim = 4;
  const sx = from.x + ux * trim, sy = from.y + uy * trim;
  const ex = to.x - ux * trim,   ey = to.y - uy * trim;

  const color = kind === 'conformist' ? 'var(--g-fg-4)' : 'var(--g-hairline-strong)';
  const dash  = kind === 'conformist' ? '3 3' : null;

  // Arrowhead at end
  const head = (() => {
    const ah = 6;
    const ax = ex - ux * ah - uy * (ah * 0.55);
    const ay = ey - uy * ah + ux * (ah * 0.55);
    const bx = ex - ux * ah + uy * (ah * 0.55);
    const by = ey - uy * ah - ux * (ah * 0.55);
    return `M ${ex} ${ey} L ${ax} ${ay} L ${bx} ${by} Z`;
  })();

  // ACL midpoint notch (triangle pointing back toward "from")
  const notch = (() => {
    const nh = 5;
    const px = -uy, py = ux; // perpendicular
    const tip   = { x: mid.x + ux * nh, y: mid.y + uy * nh };
    const left  = { x: mid.x + px * nh, y: mid.y + py * nh };
    const right = { x: mid.x - px * nh, y: mid.y - py * nh };
    return `M ${tip.x} ${tip.y} L ${left.x} ${left.y} L ${right.x} ${right.y} Z`;
  })();

  return (
    <>
      <line x1={sx} y1={sy} x2={ex} y2={ey} stroke={color} strokeWidth="1" strokeDasharray={dash || undefined} />
      {kind !== 'shared-kernel' && <path d={head} fill={color} />}
      {kind === 'anti-corruption' && <path d={notch} fill="var(--g-surface-1)" stroke={color} strokeWidth="1" />}
    </>
  );
};

// ───────── Project frame ─────────
const ProjectFrame = ({
  name, title = name, x = 0, y = 0, w = 360, h = 240,
  counts = { running: 0, blocked: 0, missing: 0, idle: 0 },
  taskLine = '0 tasks',
  focused = false,
  dim = false,
  children,
}) => {
  return (
    <div
      data-project={name}
      style={{
        position: 'absolute', left: x, top: y, width: w, height: h,
        background: 'var(--g-surface-1)',
        border: `1px solid ${focused ? 'var(--g-focus-ring)' : (dim ? 'var(--g-periwinkle-faint)' : 'var(--g-periwinkle-soft)')}`,
        borderRadius: 'var(--g-r-frame)',
        opacity: dim ? 0.72 : 1,
        transition: `border-color var(--g-camera) var(--g-ease), opacity var(--g-camera) var(--g-ease)`,
      }}
    >
      {/* Header bar */}
      <div style={{
        height: 36,
        display: 'flex', alignItems: 'center', gap: 10,
        padding: '0 12px',
        borderBottom: '1px solid var(--g-hairline)',
      }}>
        {/* Drag handle */}
        <span aria-hidden style={{
          width: 8, height: 14, flex: '0 0 8px',
          backgroundImage: 'radial-gradient(circle, var(--g-fg-4) 1px, transparent 1.2px)',
          backgroundSize: '4px 4px', backgroundPosition: '0 0',
          opacity: 0.6,
        }} />
        {/* Name */}
        <span style={{ fontSize: 16, fontWeight: 500, color: 'var(--g-fg-1)', letterSpacing: '-0.01em' }}>{title}</span>
        {/* Status mini-row */}
        <span style={{ display: 'inline-flex', gap: 10, marginLeft: 12 }}>
          <StatusMini status="running" count={counts.running} />
          <StatusMini status="blocked" count={counts.blocked} />
          <StatusMini status="missing" count={counts.missing} />
          <StatusMini status="idle"    count={counts.idle} />
        </span>
        {/* Task-count row */}
        <span className="mono" style={{ marginLeft: 'auto', fontSize: 10, color: 'var(--g-fg-3)' }}>
          {taskLine}
        </span>
      </div>
      {/* Body */}
      <div style={{ position: 'relative', width: '100%', height: h - 36 }}>
        {children}
      </div>
    </div>
  );
};

// ───────── Voice indicator (screen-space, bottom-right of viewport) ─────────
const VoiceIndicator = ({ state = 'idle', style = {} }) => {
  // states: idle (hollow ring), listening (filled pulsing), muted (slashed)
  const ring = {
    idle:      'var(--g-fg-3)',
    listening: 'var(--g-status-running)',
    muted:     'var(--g-fg-4)',
    speaking:  'var(--g-periwinkle)',
  }[state];
  const label = { idle: 'Bob', listening: 'listening', muted: 'muted', speaking: 'speaking' }[state];
  return (
    <div style={{
      position: 'absolute', right: 16, bottom: 16,
      display: 'flex', alignItems: 'center', gap: 10,
      padding: '7px 12px 7px 10px',
      background: 'var(--g-surface-1)',
      border: '1px solid var(--g-hairline)',
      borderRadius: 'var(--g-r-pill)',
      ...style,
    }}>
      <div style={{ position: 'relative', width: 16, height: 16, display: 'grid', placeItems: 'center' }}>
        <div style={{
          width: 10, height: 10, borderRadius: '50%',
          background: state === 'listening' || state === 'speaking' ? ring : 'transparent',
          border: state === 'listening' || state === 'speaking' ? 'none' : `1.25px solid ${ring}`,
          animation: state === 'listening' ? 'g-pulse-running var(--g-pulse) var(--g-ease) infinite' : 'none',
        }} />
        {/* Muted slash */}
        {state === 'muted' && (
          <svg width="18" height="18" viewBox="0 0 18 18" style={{ position: 'absolute', inset: -1 }}>
            <line x1="3" y1="15" x2="15" y2="3" stroke="var(--g-fg-2)" strokeWidth="1.4" strokeLinecap="round" />
          </svg>
        )}
        {/* Speaking waveform (subtle) */}
        {state === 'speaking' && (
          <svg width="22" height="22" viewBox="0 0 22 22" style={{ position: 'absolute', inset: -3 }}>
            <circle cx="11" cy="11" r="9" fill="none" stroke="var(--g-periwinkle)" strokeWidth="1" strokeDasharray="2 3" opacity="0.55" />
          </svg>
        )}
      </div>
      <span style={{ fontSize: 11, color: 'var(--g-fg-2)', letterSpacing: '0.01em' }}>{label}</span>
    </div>
  );
};

// ───────── Rationale plate (annotation block above each artboard) ─────────
const RationalePlate = ({ tag, title, why, decisions }) => (
  <div style={{
    width: '100%', padding: '14px 18px 16px',
    background: '#fbfaf6',
    borderRadius: 8,
    border: '1px solid #e6e2d8',
    color: '#3a342a',
    fontFamily: '-apple-system, "Segoe UI", system-ui, sans-serif',
    marginBottom: 12,
  }}>
    <div style={{ display: 'flex', alignItems: 'baseline', gap: 10 }}>
      <span style={{ fontFamily: 'ui-monospace, "Cascadia Code", monospace', fontSize: 10, color: '#9b8a6b', letterSpacing: '0.06em' }}>{tag}</span>
      <span style={{ fontSize: 14, fontWeight: 600 }}>{title}</span>
    </div>
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16, marginTop: 10, fontSize: 12, lineHeight: 1.5 }}>
      <div>
        <div style={{ fontSize: 10, color: '#9b8a6b', textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 4 }}>Why this serves one dev managing many agents</div>
        <div style={{ color: '#3a342a' }}>{why}</div>
      </div>
      <div>
        <div style={{ fontSize: 10, color: '#9b8a6b', textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 4 }}>Pixel decisions</div>
        <div style={{ color: '#3a342a', fontFamily: 'ui-monospace, "Cascadia Code", monospace', fontSize: 11, lineHeight: 1.55 }}>{decisions}</div>
      </div>
    </div>
  </div>
);

// Expose to global scope so other Babel scripts can use these atoms.
Object.assign(window, {
  StatusGlyph, StatusBadge, StatusMini,
  BCBubble, DDDEdge, ProjectFrame,
  VoiceIndicator, RationalePlate,
});
