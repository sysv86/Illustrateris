# Motion Engineering & Anti-Slop Reference

## 1. Analytical Critically-Damped Spring (Frame-Rate Independent)

Never use CSS `linear` or bouncy elastic easings on structural UI. When driving interactive motion in JavaScript/WebGPU, use an exact analytical critically-damped harmonic oscillator (`zeta = 1.0`) so position and velocity are identical at 60Hz, 120Hz, and 240Hz:

```javascript
/**
 * Advances a 1D critically-damped spring by dt seconds.
 * @param {number} x - current value
 * @param {number} v - current velocity
 * @param {number} target - target value
 * @param {number} omega - natural frequency (rad/s, e.g. 18.0 for snappy UI, 10.0 for smooth reveal)
 * @param {number} dt - delta time in seconds
 */
export function stepCriticallyDampedSpring(x, v, target, omega, dt) {
  const dx = x - target;
  const exp = Math.exp(-omega * dt);
  const c1 = dx;
  const c2 = v + omega * dx;
  const nextX = target + (c1 + c2 * dt) * exp;
  const nextV = (c2 - omega * (c1 + c2 * dt)) * exp;
  return [nextX, nextV];
}
```

## 2. CSS Motion Tokens & Progressive Enhancement

```css
:root {
  --ease-out-quart: cubic-bezier(0.22, 1, 0.36, 1);
  --ease-in-out-quart: cubic-bezier(0.76, 0, 0.24, 1);
  --dur-micro: 120ms;
  --dur-ui: 260ms;
  --dur-reveal: 650ms;
}

@media (prefers-reduced-motion: reduce) {
  *, ::before, ::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

## 3. Complete 3-Wave AI Slop Detection Matrix

| Wave | Tell / Pattern | Why It Fails | First-Principles Replacement |
| --- | --- | --- | --- |
| **Wave 1 (2023-2024)** | Violet/indigo/cyan linear gradients (`#6366f1`, `#8b5cf6`) & glowing orbs | Statistical mean of Tailwind v3 SaaS templates; zero brand specificity. | Perceptual OKLCH palette synthesized from domain hue with verified APCA `\|Lc\|` targets. |
| **Wave 1 (2023-2024)** | 3 equal cards with icons in rounded squares | Treats all features as identical weight; wastes horizontal measure. | Asymmetric 12-column grid sized by true glyph extents and visual mass (`M = InkArea * \|Lc\| * Weight`). |
| **Wave 1 (2023-2024)** | Buzzword copy (`Unleash`, `Elevate`, `Seamless`, `Next-Gen`, `Delve`, `Game-changer`) | Zero information density; fails the Competitor Swap Test immediately. | Concrete technical claims backed by verifiable metrics or interactive proof. |
| **Wave 2 (2025)** | Eyebrow pill badge above every heading (`✨ NEW`, `01 // FEATURES`) | Repetitive crutch used when typographic scale hierarchy is too weak to signal section starts. | Extreme scale contrast (`>= 4.5x` display-to-body ratio) and macro-whitespace (`120px-180px` vertical section rhythm). |
| **Wave 2 (2025)** | Fake dashboard made of empty grey bars and CSS circles | Ornament pretending to be product proof. | Real interactive component, real data table with `tabular-nums`, or a real WebGPU compute/shader surface. |
| **Wave 2 (2025)** | Uniform `fade-up` on every DOM node | Signals automated bulk-tagging rather than intentional choreography. | One signature interactive/WebGPU moment + reading-order stagger (`40-70ms`) on section entry. |
| **Wave 3 (2025-2026)** | Warm cream `#FDFBF7` + terracotta/rust + random italic serif word + Wikimedia engraving | The "anti-slop template" look created by static direction-dealing scripts (`taste-skill`). | Derive theme, font personality (`sans`, `serif`, `mono`), and chromatic energy mathematically from the subject's actual domain and tone vector. |
