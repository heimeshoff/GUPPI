// guppi-voice-views.jsx — voice indicator in idle/listening/muted, in situ.

// Mini canvas background that the indicator floats over.
const MicroCanvas = ({ children, label }) => (
  <CanvasGrid w={400} h={300}>
    <div style={{ position: 'absolute', left: 16, top: 12, color: 'var(--g-fg-4)', fontSize: 10, fontFamily: 'var(--g-font-mono)' }}>
      {label}
    </div>
    {/* A faded background tile so the indicator has visual context */}
    <MiniProjectTile x={40} y={70} w={200} h={120} scale={0.78}
      name="image-gallery"
      counts={{ running: 1, blocked: 0, missing: 0, idle: 2 }}
      bcs={[{ name: 'media', status: 'running' }, { name: 'albums', status: 'idle' }, { name: 'sharing', status: 'idle' }]}
      dim />
    <MiniProjectTile x={250} y={120} w={140} h={90} scale={0.7}
      name="auth-svc"
      counts={{ running: 1, blocked: 0, missing: 0, idle: 1 }}
      bcs={[{ name: 'identity', status: 'running' }, { name: 'audit', status: 'idle' }]}
      dim />
    {children}
  </CanvasGrid>
);

// idle: small ring, label "Bob"
const ViewVoiceIdle = () => (
  <MicroCanvas label="voice · idle">
    <VoiceIndicator state="idle" />
  </MicroCanvas>
);

// listening: pulsing dot, optional transient transcript
const ViewVoiceListening = () => (
  <MicroCanvas label="voice · listening · transcript overlay">
    {/* Transient transcript */}
    <div style={{
      position: 'absolute', right: 16, bottom: 56,
      maxWidth: 320,
      padding: '8px 12px',
      background: 'rgba(28,28,36,.94)',
      border: '1px solid var(--g-hairline-strong)',
      borderRadius: 8,
      fontSize: 12,
      color: 'var(--g-fg-1)',
      lineHeight: 1.45,
    }}>
      <span style={{ color: 'var(--g-fg-3)' }}>"</span>
      Bob, refine task 104 in image-gallery
      <span style={{ color: 'var(--g-fg-3)' }}>"</span>
      <span className="mono" style={{ marginLeft: 8, fontSize: 10, color: 'var(--g-fg-4)' }}>· 96%</span>
    </div>
    <VoiceIndicator state="listening" />
  </MicroCanvas>
);

// muted: slashed glyph
const ViewVoiceMuted = () => (
  <MicroCanvas label="voice · muted">
    <VoiceIndicator state="muted" />
  </MicroCanvas>
);

// bonus: speaking (TTS) — non-intrusive "the app is speaking" state
const ViewVoiceSpeaking = () => (
  <MicroCanvas label="voice · speaking (TTS narration)">
    <div style={{
      position: 'absolute', right: 16, bottom: 56,
      maxWidth: 320,
      padding: '8px 12px',
      background: 'rgba(28,28,36,.94)',
      border: '1px solid var(--g-periwinkle-faint)',
      borderRadius: 8,
      fontSize: 12,
      color: 'var(--g-fg-2)',
      lineHeight: 1.45,
      display: 'flex', alignItems: 'center', gap: 10,
    }}>
      <SpeakingBars />
      <span>Task 104 is queued behind two running tasks in <span style={{ color: 'var(--g-fg-1)' }}>media</span>.</span>
    </div>
    <VoiceIndicator state="speaking" />
  </MicroCanvas>
);

// A 3-bar speaking visualizer — VERY restrained, slow.
const SpeakingBars = () => (
  <svg width="14" height="14" viewBox="0 0 14 14" style={{ flex: '0 0 14px' }}>
    {[2, 6, 10].map((x, i) => (
      <rect key={i} x={x - 1} y="4" width="2" height="6" rx="1" fill="var(--g-periwinkle)" opacity={0.5 + i * 0.15}>
        <animate attributeName="height" values="3;7;3" dur={`${1.4 + i * 0.18}s`} repeatCount="indefinite" />
        <animate attributeName="y" values="5.5;3.5;5.5" dur={`${1.4 + i * 0.18}s`} repeatCount="indefinite" />
      </rect>
    ))}
  </svg>
);

Object.assign(window, { MicroCanvas, ViewVoiceIdle, ViewVoiceListening, ViewVoiceMuted, ViewVoiceSpeaking });
