# Modality Calculus & Materiality Reference

## 1. Perceptual Noisy Gradient (OKLCH Centroid Field + SVG `feTurbulence`)

Place this zero-network SVG filter once at the top of `<body>`:

```html
<svg width="0" height="0" style="position:absolute;pointer-events:none" aria-hidden="true">
  <defs>
    <filter id="ui-grain" x="0%" y="0%" width="100%" height="100%">
      <feTurbulence type="fractalNoise" baseFrequency="0.82" numOctaves="3" stitchTiles="stitch" result="noise" />
      <feColorMatrix type="saturate" values="0" in="noise" result="mono" />
      <feComponentTransfer in="mono">
        <feFuncA type="linear" slope="0.035" />
      </feComponentTransfer>
    </filter>
  </defs>
</svg>
```

Apply the atmospheric field anchored at the section's visual mass centroid `(--focal-x, --focal-y)`:

```css
.ui-atmosphere {
  position: relative;
  isolation: isolate;
  background-color: var(--bg);
  background-image:
    radial-gradient(
      115% 90% at var(--focal-x, 66%) var(--focal-y, 32%),
      oklch(from var(--accent) calc(l * 0.45) calc(c * 0.75) h / 0.18) 0%,
      oklch(from var(--surface) l c h / 0.55) 52%,
      transparent 100%
    );
}

.ui-atmosphere::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  z-index: 0;
  filter: url(#ui-grain);
  mix-blend-mode: overlay;
}

.ui-atmosphere > * {
  position: relative;
  z-index: 1;
}
```

## 2. Specular Surface Card + Micro-Interaction Physics

Instead of heavy drop shadows, use a 1px structural border (`|Lc| >= 12`), top specular inset highlight, and a 160ms compositor lift with directional SVG glyph motion:

```css
.ui-card-interactive {
  position: relative;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius, 6px);
  box-shadow: inset 0 1px 0 0 oklch(1 0 0 / 0.05);
  transition:
    transform 160ms cubic-bezier(0.22, 1, 0.36, 1),
    border-color 160ms cubic-bezier(0.22, 1, 0.36, 1);
}

.ui-card-interactive:hover {
  transform: translate3d(0, -2px, 0);
  border-color: var(--accent);
}

.ui-action-arrow {
  display: inline-block;
  transition: transform 150ms cubic-bezier(0.22, 1, 0.36, 1);
}

a:hover .ui-action-arrow,
button:hover .ui-action-arrow {
  transform: translate3d(3px, -2px, 0);
}
```

## 3. Two-Tone Parametric SVG Micro-Schematics

Every custom SVG icon or schematic must use **two optical layers**: muted structural geometry (`stroke-opacity="0.3"`) and an active focal vector (`stroke="var(--accent)"` or `opacity="1"`):

```html
<!-- Latency / Finality Topology Glyph -->
<svg width="28" height="28" viewBox="0 0 28 28" fill="none" aria-hidden="true">
  <!-- Layer 1: Structural reference grid (low contrast) -->
  <path d="M4 22H24M4 14H24M4 6H24" stroke="currentColor" stroke-opacity="0.22" stroke-width="1.25" stroke-dasharray="2 2"/>
  <!-- Layer 2: Active signal trajectory (high contrast + accent node) -->
  <path d="M4 20L11 14L17 16L24 7" stroke="currentColor" stroke-width="1.5" stroke-linecap="square" stroke-linejoin="miter"/>
  <circle cx="24" cy="7" r="2.5" fill="var(--accent)" />
</svg>
```
