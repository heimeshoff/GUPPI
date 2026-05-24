// guppi-canvas-views.jsx — overview density (1/3/10 projects) + zoom transition.

// ---------- A small inline project tile, sized by `scale` ----------
// We don't reuse ProjectFrame here because at small scales we need a more
// compressed header (no "12·3·2" line, smaller name).
const MiniProjectTile = ({ x, y, w, h, name, counts, bcs = [], dim = false, focused = false, scale = 1 }) => {
  const fontTitle  = Math.max(9,  11 * scale);
  const fontMono   = Math.max(8,  9  * scale);
  const headerH    = Math.max(20, 26 * scale);
  return (
    <div style={{
      position: 'absolute', left: x, top: y, width: w, height: h,
      background: 'var(--g-surface-1)',
      border: `1px solid ${focused ? 'var(--g-focus-ring)' : (dim ? 'var(--g-periwinkle-faint)' : 'var(--g-periwinkle-soft)')}`,
      borderRadius: 8,
      opacity: dim ? 0.6 : 1,
    }}>
      {/* Header */}
      <div style={{
        height: headerH, display: 'flex', alignItems: 'center',
        gap: 6 * scale, padding: `0 ${8 * scale}px`,
        borderBottom: '1px solid var(--g-hairline)',
      }}>
        <span style={{ fontSize: fontTitle, color: 'var(--g-fg-1)', letterSpacing: '-0.005em', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{name}</span>
        <span style={{ marginLeft: 'auto', display: 'flex', gap: 6, fontFamily: 'var(--g-font-mono)', fontSize: fontMono }}>
          {counts.running > 0 && <span style={{ color: 'var(--g-status-running)' }}>▶{counts.running}</span>}
          {counts.blocked > 0 && <span style={{ color: 'var(--g-status-blocked)' }}>◆{counts.blocked}</span>}
          {counts.missing > 0 && <span style={{ color: 'var(--g-status-missing)' }}>✕{counts.missing}</span>}
          {counts.idle > 0 && <span style={{ color: 'var(--g-fg-3)' }}>○{counts.idle}</span>}
        </span>
      </div>
      {/* Body — BC dots */}
      <div style={{ position: 'relative', padding: 8 * scale, height: h - headerH }}>
        <div style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${Math.max(2, Math.min(3, Math.ceil(bcs.length / 2)))}, 1fr)`,
          gap: 6 * scale,
          height: '100%',
          alignContent: 'start',
        }}>
          {bcs.map((bc, i) => (
            <div key={i} style={{
              background: 'var(--g-surface-2)',
              border: `1px solid var(--g-teal-soft)`,
              borderRadius: 4,
              padding: `${4 * scale}px ${6 * scale}px`,
              display: 'flex', alignItems: 'center', justifyContent: 'space-between',
              gap: 4,
              minHeight: 18 * scale,
              animation: bc.status === 'running' ? `g-pulse-running var(--g-pulse) var(--g-ease) infinite` : 'none',
            }}>
              <span style={{ fontSize: Math.max(8, 9.5 * scale), color: 'var(--g-fg-2)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{bc.name}</span>
              <StatusGlyph status={bc.status} size={Math.max(6, 7 * scale)} />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

// ---------- Backdrop wrapper ----------
const CanvasGrid = ({ children, w = 800, h = 500 }) => (
  <div className="guppi g-canvas-grid" style={{
    width: w, height: h, position: 'relative', overflow: 'hidden',
    borderRadius: 6, border: '1px solid var(--g-canvas-border)',
  }}>
    {children}
  </div>
);

// ───────── 1 project — the workspace at first run ─────────
const View1Project = () => (
  <CanvasGrid w={820} h={540}>
    {/* Centered hint */}
    <div style={{
      position: 'absolute', left: 36, top: 36, color: 'var(--g-fg-4)', fontSize: 11,
      fontFamily: 'var(--g-font-mono)',
    }}>
      workspace · 1 project
    </div>
    <MiniProjectTile
      x={210} y={150} w={320} h={210} scale={1.2}
      name="image-gallery"
      counts={{ running: 1, blocked: 0, missing: 0, idle: 3 }}
      bcs={[
        { name: 'media',      status: 'running' },
        { name: 'albums',     status: 'idle' },
        { name: 'sharing',    status: 'idle' },
        { name: 'thumbnails', status: 'idle' },
      ]}
    />
    {/* Hint affordance bottom-center */}
    <div style={{
      position: 'absolute', left: '50%', bottom: 30, transform: 'translateX(-50%)',
      display: 'flex', alignItems: 'center', gap: 10,
      fontSize: 11, color: 'var(--g-fg-3)', fontFamily: 'var(--g-font-sans)',
    }}>
      <span className="mono" style={{ padding: '2px 6px', border: '1px solid var(--g-hairline-strong)', borderRadius: 4, fontSize: 10 }}>⌘N</span>
      <span>new project</span>
      <span style={{ color: 'var(--g-fg-4)', margin: '0 6px' }}>·</span>
      <span className="mono" style={{ padding: '2px 6px', border: '1px solid var(--g-hairline-strong)', borderRadius: 4, fontSize: 10 }}>⌘K</span>
      <span>commands</span>
      <span style={{ color: 'var(--g-fg-4)', margin: '0 6px' }}>·</span>
      <span style={{ color: 'var(--g-fg-3)' }}>or say <span style={{ color: 'var(--g-periwinkle)' }}>"Bob"</span></span>
    </div>
    <VoiceIndicator state="idle" />
  </CanvasGrid>
);

// ───────── 3 projects — comfortable density ─────────
const View3Projects = () => (
  <CanvasGrid w={820} h={540}>
    <div style={{ position: 'absolute', left: 36, top: 36, color: 'var(--g-fg-4)', fontSize: 11, fontFamily: 'var(--g-font-mono)' }}>
      workspace · 3 projects
    </div>
    <MiniProjectTile x={90}  y={110} w={280} h={170} scale={1}
      name="image-gallery"
      counts={{ running: 1, blocked: 1, missing: 0, idle: 2 }}
      bcs={[
        { name: 'media',      status: 'running' },
        { name: 'albums',     status: 'blocked' },
        { name: 'sharing',    status: 'idle' },
        { name: 'thumbnails', status: 'idle' },
      ]} />
    <MiniProjectTile x={440} y={90} w={280} h={170} scale={1}
      name="auth-service"
      counts={{ running: 2, blocked: 0, missing: 0, idle: 1 }}
      bcs={[
        { name: 'identity',    status: 'running' },
        { name: 'sessions',    status: 'running' },
        { name: 'permissions', status: 'idle' },
      ]} />
    <MiniProjectTile x={260} y={320} w={280} h={170} scale={1}
      name="ml-trainer"
      counts={{ running: 0, blocked: 0, missing: 1, idle: 3 }}
      bcs={[
        { name: 'dataset',     status: 'idle' },
        { name: 'training',    status: 'idle' },
        { name: 'inference',   status: 'idle' },
        { name: 'checkpoints', status: 'missing' },
      ]} />
    <VoiceIndicator state="idle" />
  </CanvasGrid>
);

// ───────── 10 projects — dense workspace ─────────
const View10Projects = () => {
  const tiles = [
    { name: 'image-gallery',   counts: { running: 1, blocked: 1, missing: 0, idle: 2 }, bcs: [
        { name: 'media', status: 'running' }, { name: 'albums', status: 'blocked' },
        { name: 'sharing', status: 'idle' }, { name: 'users', status: 'idle' } ] },
    { name: 'auth-service',    counts: { running: 2, blocked: 0, missing: 0, idle: 1 }, bcs: [
        { name: 'identity', status: 'running' }, { name: 'sessions', status: 'running' },
        { name: 'audit', status: 'idle' } ] },
    { name: 'ml-trainer',      counts: { running: 0, blocked: 0, missing: 1, idle: 3 }, bcs: [
        { name: 'dataset', status: 'idle' }, { name: 'training', status: 'idle' },
        { name: 'inference', status: 'idle' }, { name: 'checkpoints', status: 'missing' } ] },
    { name: 'crawler',         counts: { running: 1, blocked: 0, missing: 0, idle: 2 }, bcs: [
        { name: 'fetch', status: 'running' }, { name: 'parse', status: 'idle' },
        { name: 'store', status: 'idle' } ] },
    { name: 'payment-rails',   counts: { running: 0, blocked: 2, missing: 0, idle: 1 }, bcs: [
        { name: 'ledger', status: 'blocked' }, { name: 'webhook', status: 'blocked' },
        { name: 'fx', status: 'idle' } ] },
    { name: 'doc-site',        counts: { running: 0, blocked: 0, missing: 0, idle: 3 }, bcs: [
        { name: 'content', status: 'idle' }, { name: 'theme', status: 'idle' },
        { name: 'search', status: 'idle' } ] },
    { name: 'analytics-pipe',  counts: { running: 3, blocked: 0, missing: 0, idle: 0 }, bcs: [
        { name: 'ingest', status: 'running' }, { name: 'rollup', status: 'running' },
        { name: 'export', status: 'running' } ] },
    { name: 'search-index',    counts: { running: 0, blocked: 1, missing: 0, idle: 2 }, bcs: [
        { name: 'index', status: 'blocked' }, { name: 'query', status: 'idle' },
        { name: 'cache', status: 'idle' } ] },
    { name: 'mobile-app',      counts: { running: 0, blocked: 0, missing: 0, idle: 3 }, bcs: [
        { name: 'ui', status: 'idle' }, { name: 'sync', status: 'idle' },
        { name: 'push', status: 'idle' } ] },
    { name: 'cli-tool',        counts: { running: 1, blocked: 0, missing: 0, idle: 1 }, bcs: [
        { name: 'core', status: 'running' }, { name: 'plugins', status: 'idle' } ] },
  ];

  // Loose grid, slightly scattered (positions feel placed by the user).
  const layout = [
    [50, 90],  [300, 70], [560, 100],
    [70, 240], [320, 230], [580, 250],
    [110, 380], [370, 400], [610, 390],
    [200, 545]  // off-canvas, won't show — drop this one
  ];

  return (
    <CanvasGrid w={820} h={540}>
      <div style={{ position: 'absolute', left: 36, top: 16, color: 'var(--g-fg-4)', fontSize: 11, fontFamily: 'var(--g-font-mono)' }}>
        workspace · 10 projects · 31 BCs
      </div>
      {tiles.slice(0, 9).map((t, i) => (
        <MiniProjectTile key={t.name} x={layout[i][0]} y={layout[i][1]} w={220} h={130} scale={0.82}
          name={t.name} counts={t.counts} bcs={t.bcs} />
      ))}
      {/* 10th tile, shown partially off-edge to suggest the canvas extends */}
      <MiniProjectTile x={690} y={420} w={220} h={130} scale={0.82}
        name={tiles[9].name} counts={tiles[9].counts} bcs={tiles[9].bcs} dim />
      <VoiceIndicator state="idle" />
    </CanvasGrid>
  );
};

// ───────── Zoom transition: overview vs focused ─────────
const ViewZoomOverview = () => {
  const tiles = [
    { name: 'image-gallery',  counts: { running: 1, blocked: 1, missing: 0, idle: 2 }, bcs: [{name:'media',status:'running'},{name:'albums',status:'blocked'},{name:'sharing',status:'idle'},{name:'users',status:'idle'}], focus: true },
    { name: 'auth-service',   counts: { running: 2, blocked: 0, missing: 0, idle: 1 }, bcs: [{name:'identity',status:'running'},{name:'sessions',status:'running'},{name:'audit',status:'idle'}] },
    { name: 'analytics-pipe', counts: { running: 3, blocked: 0, missing: 0, idle: 0 }, bcs: [{name:'ingest',status:'running'},{name:'rollup',status:'running'},{name:'export',status:'running'}] },
    { name: 'crawler',        counts: { running: 1, blocked: 0, missing: 0, idle: 2 }, bcs: [{name:'fetch',status:'running'},{name:'parse',status:'idle'},{name:'store',status:'idle'}] },
    { name: 'payment-rails',  counts: { running: 0, blocked: 2, missing: 0, idle: 1 }, bcs: [{name:'ledger',status:'blocked'},{name:'webhook',status:'blocked'},{name:'fx',status:'idle'}] },
    { name: 'ml-trainer',     counts: { running: 0, blocked: 0, missing: 1, idle: 3 }, bcs: [{name:'dataset',status:'idle'},{name:'training',status:'idle'},{name:'checkpoints',status:'missing'},{name:'inference',status:'idle'}] },
  ];
  const layout = [[90,90],[370,90],[630,110],[110,260],[380,280],[620,300]];
  return (
    <CanvasGrid w={780} h={520}>
      <div style={{ position: 'absolute', left: 24, top: 14, color: 'var(--g-fg-4)', fontSize: 10, fontFamily: 'var(--g-font-mono)' }}>
        zoom 38% · overview
      </div>
      {tiles.map((t, i) => (
        <MiniProjectTile key={t.name} x={layout[i][0]} y={layout[i][1]} w={220} h={140} scale={0.85}
          name={t.name} counts={t.counts} bcs={t.bcs} focused={t.focus} dim={!t.focus && i > 2} />
      ))}
      {/* Focus ring annotation */}
      <div style={{ position: 'absolute', left: 90, top: 240, fontSize: 10, color: 'var(--g-focus-ring)', fontFamily: 'var(--g-font-mono)' }}>
        ↑ focused frame
      </div>
      <VoiceIndicator state="idle" />
    </CanvasGrid>
  );
};

const ViewZoomFocused = () => (
  <CanvasGrid w={780} h={520}>
    <div style={{ position: 'absolute', left: 24, top: 14, color: 'var(--g-fg-4)', fontSize: 10, fontFamily: 'var(--g-font-mono)' }}>
      zoom 110% · image-gallery
    </div>
    {/* The same focused project, now filling the viewport */}
    <ProjectFrame
      name="image-gallery" title="image-gallery"
      x={50} y={50} w={680} h={420}
      counts={{ running: 1, blocked: 1, missing: 0, idle: 2 }}
      taskLine="12 tasks · 3 active · 2 blocked"
      focused
    >
      <BCBubble name="media"      status="running" running={2} blocked={0} x={28}  y={28}  w={186} h={56} />
      <BCBubble name="albums"     status="blocked" running={0} blocked={1} idle={2} x={244} y={28}  w={186} h={56} />
      <BCBubble name="sharing"    status="idle"    idle={1}    x={460} y={28}  w={186} h={56} />
      <BCBubble name="thumbnails" status="running" running={1} x={28}  y={140} w={186} h={56} />
      <BCBubble name="users"      status="idle"    idle={2}    x={244} y={140} w={186} h={56} />
      <BCBubble name="uploads"    status="idle"    idle={1}    x={460} y={140} w={186} h={56} />
      <BCBubble name="search"     status="idle"    idle={1}    x={244} y={252} w={186} h={56} />
      {/* Edges */}
      <svg width="652" height="356" style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}>
        <DDDEdge from={{ x: 214, y: 56 }} to={{ x: 244, y: 56 }} kind="customer-supplier" />
        <DDDEdge from={{ x: 430, y: 56 }} to={{ x: 460, y: 56 }} kind="shared-kernel" />
        <DDDEdge from={{ x: 121, y: 84 }} to={{ x: 121, y: 140 }} kind="customer-supplier" />
        <DDDEdge from={{ x: 337, y: 84 }} to={{ x: 337, y: 252 }} kind="anti-corruption" />
        <DDDEdge from={{ x: 430, y: 168 }} to={{ x: 460, y: 168 }} kind="conformist" />
      </svg>
    </ProjectFrame>
    <VoiceIndicator state="idle" />
  </CanvasGrid>
);

Object.assign(window, {
  MiniProjectTile, CanvasGrid,
  View1Project, View3Projects, View10Projects,
  ViewZoomOverview, ViewZoomFocused,
});
