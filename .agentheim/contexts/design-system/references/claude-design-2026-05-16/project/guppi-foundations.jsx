// guppi-foundations.jsx — at-a-glance design system reference.

// Read a CSS variable from :root and refresh when [data-theme] changes,
// so the swatch labels stay accurate as the user toggles light/dark.
const useThemeVar = (names) => {
  const [vals, setVals] = React.useState({});
  React.useEffect(() => {
    const read = () => {
      const cs = getComputedStyle(document.documentElement);
      const out = {};
      names.forEach((n) => { out[n] = cs.getPropertyValue(n).trim().toUpperCase(); });
      setVals(out);
    };
    read();
    const obs = new MutationObserver(read);
    obs.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => obs.disconnect();
  }, [names.join('|')]);
  return vals;
};

const ViewFoundations = () => {
  const v = useThemeVar([
    '--g-bg', '--g-surface-1', '--g-surface-2', '--g-surface-3',
    '--g-hairline', '--g-hairline-strong',
    '--g-periwinkle', '--g-teal', '--g-focus-ring',
    '--g-status-idle', '--g-status-running', '--g-status-blocked', '--g-status-missing',
  ]);
  return (
  <div className="guppi" style={{
    width: 1240, height: 720, padding: 32,
    background: 'var(--g-bg)',
    display: 'grid',
    gridTemplateColumns: '1fr 1fr 1.1fr',
    gridTemplateRows: 'auto 1fr',
    gridTemplateAreas: '"head head head" "palette type status"',
    gap: 28,
    borderRadius: 6,
    border: '1px solid var(--g-canvas-border)',
  }}>
    <div style={{ gridArea: 'head' }}>
      <div className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', letterSpacing: '0.1em', textTransform: 'uppercase', marginBottom: 6 }}>
        guppi · foundations
      </div>
      <div style={{ fontSize: 24, color: 'var(--g-fg-1)', letterSpacing: '-0.015em', maxWidth: 740 }}>
        Geometry, not chrome, carries meaning.
      </div>
      <div style={{ fontSize: 12, color: 'var(--g-fg-3)', marginTop: 6, maxWidth: 740, lineHeight: 1.5 }}>
        Two accent hues, four status colors, one font, three sizes, 4px scale, one ambient animation.
        Everything else is hairline borders and surface ladder. Avoiding shadows, gradients, or decorative borders is what lets fifty frames feel calm.
      </div>
    </div>

    {/* Palette */}
    <Block area="palette" title="Surfaces + accents">
      <Swatch hex={v['--g-bg']}            name="bg"             sub="canvas backdrop" />
      <Swatch hex={v['--g-surface-1']}     name="surface-1"      sub="project frame" />
      <Swatch hex={v['--g-surface-2']}     name="surface-2"      sub="BC bubble" />
      <Swatch hex={v['--g-surface-3']}     name="surface-3"      sub="panels, modals" />
      <Swatch hex={v['--g-hairline']}      name="hairline"       sub="dividers" />
      <Swatch hex={v['--g-hairline-strong']} name="hairline-strong" sub="edges, code blocks" />
      <Swatch hex={v['--g-periwinkle']}    name="accent · warm"   sub="brand orange · project frame" accent />
      <Swatch hex={v['--g-teal']}          name="accent · cool"   sub="brand blue · BC bubble + running" accent />
      <Swatch hex={v['--g-focus-ring']}    name="focus-ring"     sub="the only ornament" accent />
    </Block>

    {/* Type */}
    <Block area="type" title="Type · only three sizes">
      <div style={{ fontFamily: 'var(--g-font-sans)', fontSize: 16, color: 'var(--g-fg-1)', letterSpacing: '-0.01em' }}>
        16 · title  <span className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', marginLeft: 8 }}>Inter / Segoe UI · 500</span>
      </div>
      <div style={{ fontFamily: 'var(--g-font-sans)', fontSize: 12, color: 'var(--g-fg-1)', marginTop: 14 }}>
        12 · body  <span className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', marginLeft: 8 }}>regular · letter-spacing −0.005em</span>
      </div>
      <div className="mono" style={{ fontSize: 10, color: 'var(--g-fg-2)', marginTop: 14 }}>
        10 · caption  <span style={{ marginLeft: 8, color: 'var(--g-fg-4)' }}>Cascadia Code · tnum</span>
      </div>

      <div style={{ marginTop: 28, borderTop: '1px solid var(--g-hairline)', paddingTop: 16 }}>
        <div className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', letterSpacing: '0.06em', textTransform: 'uppercase', marginBottom: 10 }}>
          spacing · 4px scale
        </div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'baseline' }}>
          {[4, 8, 12, 16, 24, 32, 48].map(v => (
            <div key={v} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
              <div style={{ width: v, height: v, background: 'var(--g-periwinkle)', opacity: 0.4 }} />
              <span className="mono" style={{ fontSize: 9, color: 'var(--g-fg-4)' }}>{v}</span>
            </div>
          ))}
        </div>
      </div>

      <div style={{ marginTop: 24, borderTop: '1px solid var(--g-hairline)', paddingTop: 16 }}>
        <div className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', letterSpacing: '0.06em', textTransform: 'uppercase', marginBottom: 10 }}>
          motion · the only animations
        </div>
        <div style={{ fontSize: 11, color: 'var(--g-fg-2)', lineHeight: 1.7 }}>
          camera <span className="mono" style={{ color: 'var(--g-fg-1)' }}>320ms</span> ease-out cubic<br/>
          running pulse <span className="mono" style={{ color: 'var(--g-fg-1)' }}>1600ms</span> · 0→3px halo, restrained<br/>
          everything else <span className="mono" style={{ color: 'var(--g-fg-3)' }}>0ms</span>
        </div>
      </div>
    </Block>

    {/* Status */}
    <Block area="status" title="Status · color + glyph, colorblind-safe">
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>
        <StatusRow status="idle"    name="idle"    hex={v['--g-status-idle']}    gloss="nothing happening" />
        <StatusRow status="running" name="running" hex={v['--g-status-running']} gloss="agent working · slow ambient pulse" pulse />
        <StatusRow status="blocked" name="blocked" hex={v['--g-status-blocked']} gloss="waiting for your answer" />
        <StatusRow status="missing" name="missing" hex={v['--g-status-missing']} gloss="expected artifact absent" />
      </div>

      <div style={{ marginTop: 28, borderTop: '1px solid var(--g-hairline)', paddingTop: 16 }}>
        <div className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', letterSpacing: '0.06em', textTransform: 'uppercase', marginBottom: 12 }}>
          DDD edges · geometry distinguishes type
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10, fontSize: 11, color: 'var(--g-fg-2)' }}>
          <EdgeRow kind="customer-supplier" label="customer-supplier" />
          <EdgeRow kind="shared-kernel"     label="shared-kernel" />
          <EdgeRow kind="anti-corruption"   label="anti-corruption-layer" />
          <EdgeRow kind="conformist"        label="conformist" />
        </div>
        <div className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', marginTop: 14 }}>
          all edges share one neutral color · <span style={{ color: 'var(--g-fg-3)' }}>var(--g-hairline-strong)</span>
        </div>
      </div>
    </Block>
  </div>
  );
};

const Block = ({ area, title, children }) => (
  <div style={{ gridArea: area, display: 'flex', flexDirection: 'column' }}>
    <div className="mono" style={{ fontSize: 10, color: 'var(--g-fg-4)', letterSpacing: '0.08em', textTransform: 'uppercase', marginBottom: 14 }}>
      {title}
    </div>
    {children}
  </div>
);

const Swatch = ({ hex, name, sub, accent = false }) => (
  <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 8 }}>
    <div style={{ width: 28, height: 28, borderRadius: 4, background: hex, border: accent ? `1px solid ${hex}` : '1px solid var(--g-hairline)' }} />
    <span style={{ fontSize: 12, color: 'var(--g-fg-1)' }}>{name}</span>
    <span className="mono" style={{ fontSize: 10, color: 'var(--g-fg-3)', marginLeft: 'auto' }}>{hex}</span>
    <span style={{ fontSize: 10, color: 'var(--g-fg-4)', minWidth: 130, textAlign: 'right' }}>{sub}</span>
  </div>
);

const StatusRow = ({ status, name, hex, gloss, pulse = false }) => (
  <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <StatusBadge status={status} pulse={pulse} size={16} />
      <span style={{ fontSize: 12, color: 'var(--g-fg-1)' }}>{name}</span>
      <span className="mono" style={{ fontSize: 10, color: 'var(--g-fg-3)', marginLeft: 'auto' }}>{hex}</span>
    </div>
    <span style={{ fontSize: 11, color: 'var(--g-fg-3)' }}>{gloss}</span>
  </div>
);

const EdgeRow = ({ kind, label }) => (
  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
    <svg width="46" height="10" viewBox="0 0 46 10">
      <DDDEdge from={{ x: 2, y: 5 }} to={{ x: 44, y: 5 }} kind={kind} />
    </svg>
    <span>{label}</span>
  </div>
);

Object.assign(window, { ViewFoundations });
