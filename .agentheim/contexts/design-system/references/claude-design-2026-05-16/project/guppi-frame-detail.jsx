// guppi-frame-detail.jsx — single project frame detail + blocked-on-question moment.

// ───────── Single project frame, large + annotated ─────────
const ViewProjectFrameDetail = () => (
  <CanvasGrid w={820} h={560}>
    <div style={{ position: 'absolute', left: 24, top: 14, color: 'var(--g-fg-4)', fontSize: 10, fontFamily: 'var(--g-font-mono)' }}>
      project frame · atomic unit · zoom 100%
    </div>
    <ProjectFrame
      name="image-gallery" title="image-gallery"
      x={40} y={50} w={740} h={460}
      counts={{ running: 2, blocked: 1, missing: 0, idle: 3 }}
      taskLine="14 tasks · 3 active · 1 blocked · 10 idle"
    >
      {/* SVG layer for edges (drawn first so bubbles overlap them visually) */}
      <svg width="740" height="424" style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}>
        {/* customer-supplier: media → albums */}
        <DDDEdge from={{ x: 224, y: 72 }} to={{ x: 256, y: 72 }} kind="customer-supplier" />
        {/* shared-kernel: albums ↔ sharing */}
        <DDDEdge from={{ x: 466, y: 72 }} to={{ x: 498, y: 72 }} kind="shared-kernel" />
        {/* customer-supplier: media → thumbnails (downward) */}
        <DDDEdge from={{ x: 132, y: 104 }} to={{ x: 132, y: 158 }} kind="customer-supplier" />
        {/* anti-corruption: albums → users */}
        <DDDEdge from={{ x: 360, y: 104 }} to={{ x: 360, y: 268 }} kind="anti-corruption" />
        {/* conformist: sharing → users (lighter) */}
        <DDDEdge from={{ x: 590, y: 104 }} to={{ x: 460, y: 268 }} kind="conformist" />
        {/* customer-supplier: thumbnails → uploads */}
        <DDDEdge from={{ x: 224, y: 188 }} to={{ x: 256, y: 188 }} kind="customer-supplier" />
      </svg>

      {/* BCs */}
      <BCBubble name="media"      status="running" running={2}             x={40}  y={44}  w={188} h={60} />
      <BCBubble name="albums"     status="blocked" blocked={1} idle={2}    x={264} y={44}  w={196} h={60} />
      <BCBubble name="sharing"    status="idle"    idle={3}                x={500} y={44}  w={196} h={60} />
      <BCBubble name="thumbnails" status="running" running={1}             x={40}  y={158} w={188} h={60} />
      <BCBubble name="uploads"    status="idle"    idle={1}                x={264} y={158} w={196} h={60} />
      <BCBubble name="search"     status="idle"    idle={1}                x={500} y={158} w={196} h={60} />
      <BCBubble name="users"      status="missing" missing={1}             x={264} y={272} w={196} h={60} />
      <BCBubble name="audit"      status="idle"    idle={1}                x={500} y={272} w={196} h={60} />

      {/* Edge-type legend at bottom, in the frame */}
      <div style={{
        position: 'absolute', left: 28, bottom: 12,
        display: 'flex', gap: 18, fontSize: 10, color: 'var(--g-fg-3)',
        fontFamily: 'var(--g-font-sans)', alignItems: 'center',
      }}>
        <EdgeLegend kind="customer-supplier" label="customer-supplier" />
        <EdgeLegend kind="shared-kernel"     label="shared-kernel" />
        <EdgeLegend kind="anti-corruption"   label="anti-corruption-layer" />
        <EdgeLegend kind="conformist"        label="conformist" />
      </div>
    </ProjectFrame>
  </CanvasGrid>
);

const EdgeLegend = ({ kind, label }) => (
  <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
    <svg width="36" height="10" viewBox="0 0 36 10">
      <DDDEdge from={{ x: 2, y: 5 }} to={{ x: 34, y: 5 }} kind={kind} />
    </svg>
    <span>{label}</span>
  </span>
);

// ───────── Blocked-on-question moment ─────────
// Shows several BCs at neighborhood-scale, with the blocked one having
// a tethered callout rendered ON the canvas (not modal). The callout
// holds the actual question. Designed so the eye lands on it even
// against running/idle siblings.
const ViewBlockedMoment = () => (
  <CanvasGrid w={820} h={560}>
    <div style={{ position: 'absolute', left: 24, top: 14, color: 'var(--g-fg-4)', fontSize: 10, fontFamily: 'var(--g-font-mono)' }}>
      blocked moment · question rendered at the BC's location
    </div>

    {/* Project frame, focused */}
    <ProjectFrame
      name="payment-rails" title="payment-rails"
      x={40} y={50} w={500} h={460}
      counts={{ running: 1, blocked: 1, missing: 0, idle: 2 }}
      taskLine="8 tasks · 1 active · 1 blocked"
    >
      <svg width="500" height="424" style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}>
        <DDDEdge from={{ x: 218, y: 76 }} to={{ x: 256, y: 76 }} kind="customer-supplier" />
        <DDDEdge from={{ x: 124, y: 108 }} to={{ x: 124, y: 178 }} kind="customer-supplier" />
        <DDDEdge from={{ x: 360, y: 108 }} to={{ x: 360, y: 178 }} kind="anti-corruption" />
        <DDDEdge from={{ x: 218, y: 208 }} to={{ x: 256, y: 208 }} kind="shared-kernel" />
      </svg>
      <BCBubble name="ledger"   status="blocked" blocked={1}                x={36}  y={46}  w={186} h={62} focused />
      <BCBubble name="webhook"  status="running" running={1}                x={264} y={46}  w={196} h={62} />
      <BCBubble name="fx"       status="idle"    idle={2}                   x={36}  y={178} w={186} h={62} />
      <BCBubble name="reports"  status="idle"    idle={1}                   x={264} y={178} w={196} h={62} />
    </ProjectFrame>

    {/* Tether line from the blocked BC out to the callout */}
    <svg width="820" height="560" style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}>
      {/* Anchor point: right edge of the focused "ledger" BC ≈ (40 + 36 + 186, 50 + 36 + 46 + 31) = (262, 163) */}
      <path d="M 262 163 C 360 163, 480 145, 560 145" stroke="var(--g-status-blocked)" strokeWidth="1" fill="none" strokeDasharray="2 3" opacity="0.85" />
      <circle cx="262" cy="163" r="2.5" fill="var(--g-status-blocked)" />
    </svg>

    {/* The question callout */}
    <div style={{
      position: 'absolute', left: 560, top: 90,
      width: 240,
      background: 'var(--g-surface-1)',
      border: '1px solid var(--g-status-blocked)',
      borderRadius: 8,
      padding: '12px 14px 12px 14px',
      animation: `g-pulse-running var(--g-pulse) var(--g-ease) infinite`,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
        <StatusBadge status="blocked" size={14} />
        <span className="mono" style={{ fontSize: 10, color: 'var(--g-status-blocked)', letterSpacing: '0.04em', textTransform: 'uppercase' }}>blocked · ledger</span>
        <span className="mono" style={{ marginLeft: 'auto', fontSize: 10, color: 'var(--g-fg-4)' }}>2m 14s</span>
      </div>
      <div style={{ fontSize: 12, lineHeight: 1.5, color: 'var(--g-fg-1)' }}>
        Should refunds reverse the original ledger entry, or post a separate negative entry referencing it?
      </div>
      <div style={{ display: 'flex', gap: 6, marginTop: 12 }}>
        <button style={btnGhost}>reverse</button>
        <button style={btnGhost}>separate</button>
        <button style={{ ...btnGhost, marginLeft: 'auto', color: 'var(--g-fg-3)' }}>type ↵</button>
      </div>
    </div>

    {/* Secondary blocked elsewhere on the canvas (dimmer, smaller) */}
    <div style={{
      position: 'absolute', left: 600, top: 320,
      width: 200,
      background: 'var(--g-surface-1)',
      border: '1px solid var(--g-hairline-strong)',
      borderRadius: 8, padding: '10px 12px',
      opacity: 0.78,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
        <StatusBadge status="blocked" size={12} />
        <span className="mono" style={{ fontSize: 10, color: 'var(--g-status-blocked)' }}>image-gallery · albums</span>
      </div>
      <div style={{ fontSize: 11, lineHeight: 1.45, color: 'var(--g-fg-2)' }}>
        Cascade delete when an album is removed?
      </div>
    </div>

    <VoiceIndicator state="idle" />
  </CanvasGrid>
);

const btnGhost = {
  fontFamily: 'var(--g-font-sans)', fontSize: 11,
  background: 'transparent',
  color: 'var(--g-fg-1)',
  border: '1px solid var(--g-hairline-strong)',
  borderRadius: 4,
  padding: '4px 8px',
  cursor: 'pointer',
};

Object.assign(window, { ViewProjectFrameDetail, ViewBlockedMoment, EdgeLegend });
