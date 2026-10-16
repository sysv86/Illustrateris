---
name: illustrateris-aesthetic-taste
description: First-principles Aesthetic Taste, Uniqueness Synthesis, Photo/Video Engineering, and Modality Calculus for Illustrateris. Teaches the AI how to create genuine visual taste, unmistakable project uniqueness, and optimal photo/video integration (AVIF, WebP, AV1, HEVC, VP9, WebGPU video textures) without hardcoded templates. Defines the exact mathematical and perceptual triggers for noisy gradients (feTurbulence + OKLCH light fields), WebGPU shaders, micro-animations, and custom parametric SVG schematics.
---

# Illustrateris Aesthetic Taste: Uniqueness, Media & Modality Calculus

A layout that only satisfies grid geometry and contrast targets is structurally sound but perceptually dead. Conversely, template-driven skills destroy taste by forcing pre-baked JSON layouts or blanket rules ("never use gradients, never use monospace").

**Genuine design taste is the calibration of sensory signal-to-noise combined with unmistakable domain uniqueness.** It is knowing how to invent a bespoke interactive artifact that belongs only to this subject, when a surface needs stochastic grain and an OKLCH light field, when a claim demands a live WebGPU shader, how to encode and grade photo/video assets (`AVIF`, `AV1`, `HEVC`), when an interaction needs a 140ms specular micro-animation, and when a concept requires a custom parametric SVG schematic.

## When to Use

- Designing, styling, or upgrading any landing page, web interface, or component where the output must feel crafted by a senior design engineer with real taste and unmistakable uniqueness.
- Deciding which sensory modalities (**Noisy Gradient**, **WebGPU Shader**, **Photo/Video Media**, **Micro-Animation**, **SVG Icon/Schematic**) belong in each section of a page.
- Engineering responsive photography (`<picture>`, `AVIF`, `WebP`) and high-efficiency video loops (`<video>`, `AV1`, `HEVC`, `VP9`, alpha video, or WebGPU `importExternalTexture`).
- Upgrading raw or ejected `ple-engine` HTML/CSS so it has tactile materiality, optical depth, and kinetic life.

## Steps

### 1. Engineer True Uniqueness (Zero-Template Signature Synthesis)

Even mathematically perfect pages look samey if two projects in similar industries receive the same rhythm and components. To guarantee **unmistakable uniqueness** without static templates, execute this 3-part synthesis on every build:

1. **The Domain-Native Interactive Artifact (One Bespoke Live Mechanism)**:
   Every flagship page must feature **one interactive component engineered from the subject's physical or mathematical mechanism** -- something that would be nonsensical on a competitor's site in another industry:
   - *Audio / DSP / Hardware*: An interactive frequency-response filter curve or tactile rotary encoder with live SVG/WebGPU waveform feedback.
   - *Fintech / Settlement / Cryptography*: A live transaction finality timeline or interactive Merkle-proof verifier with real-time `tabular-nums` latency scrubbers.
   - *Industrial / Manufacturing / Robotics*: An orthographic CAD blueprint inspector with live tolerance callouts and coordinate readouts.
   - *Craft / Roastery / Physical Goods*: An interactive material/roast profile curve (temperature vs. time) that updates tasting notes and substrate tint.
2. **Continuous Chromatic & Substrate Divergence**:
   Never reuse fixed background hexes (`#0a0a0a` or `#fdfbf7`). Derive substrate lightness and harmonic accent angles continuously from the subject's `[precision, warmth, drama, chromatic_energy]` vector:
   - **Dark Substrates**: High-drama cinematic brands sink to deep obsidian (`oklch(0.115 C H)`), while high-precision technical tools sit on graphite-slate (`oklch(0.155 C H)`).
   - **Light Substrates**: High-warmth editorial brands sit on tactile stone/ivory (`oklch(0.972 0.012 H)`), while high-precision clinical/engineering brands sit on crisp optical white (`oklch(0.990 0.003 H)`).
   - **Harmonic Accent Angle**: Use analogous hue shifts (`+28..+65 deg`) for warm/organic subjects, cool split shifts (`+95..+135 deg`) for precision engineering, and complementary tension (`+155..+190 deg`) for high-drama launches.
3. **Asymmetric Structural Signatures**:
   Vary section aspect ratios, BSP split ratios (`0.618`, `0.667`, `0.75`), and typographic anchor points across consecutive sections. If Section 1 is a `7/5` split with a right-anchored visual, Section 2 must damp to a full-width `12`-column proof matrix or an inverted `4/8` narrative split.

### 2. Compute the Domain Vector & Modality Budget

Evaluate the subject's continuous 4D tone vector in `[0.0, 1.0]`:

- **Precision (`P`)**: Technical rigor, numerical density, structural exactness (`0.9` for settlement layers, compilers, telemetry; `0.2` for artisan goods).
- **Warmth (`W`)**: Human tactility, physical materiality, organic softness (`0.85` for hospitality, craft, editorial; `0.15` for infrastructure).
- **Drama (`D`)**: Scale contrast, spatial tension, theatrical impact (`0.9` for flagship launches, creative studios; `0.2` for utility docs).
- **Chromatic Energy (`E`)**: Perceptual chroma intensity and light-field activity (`0.0` monochrome to `1.0` high-chroma optical energy).

Run the bundled taste calculus engine to compute exact modality activation scores and CSS/SVG/media parameters:

```bash
python3 skills/illustrateris-aesthetic-taste/scripts/taste_calculus.py \
  --precision 0.85 --warmth 0.25 --drama 0.80 --energy 0.35 --dark
```

#### The Mutual Exclusion & Sensory Budget Law

1. **One Dominant Hero Modality**: Choose **one** primary hero visual driver -- either a **WebGPU/Three.js Shader**, a **High-Craft Video/Photo Composition**, or **Monumental Typography over a Noisy OKLCH Light Field**. Never stack all three on the same screen.
2. **Figure-Ground Frequency Separation**: High-frequency foreground content (dense metrics, pricing matrices, code blocks, multi-axis SVG schematics) MUST sit on low-frequency, calm surfaces. Low-frequency foreground content (a 5-word display statement or a single CTA) SHOULD sit on a high-tactility surface (noisy atmospheric gradient, graded photograph, or shader field).

### 3. When and How to Use a Noisy Gradient (`feTurbulence` + OKLCH Field)

Pure flat digital hex fills (`#0a0a0a` or `#ffffff`) across millions of pixels have **zero spatial frequency** -- the human eye reads them as cheap plastic. Meanwhile, standard CSS `linear-gradient(45deg, #6366f1, #a855f7)` is Wave 1 AI slop because sRGB linear interpolation passes through a muddy dead-zone and suffers from 8-bit Mach banding.

- **Activate when**: `D * E >= 0.18`, OR `W >= 0.50` (tactile paper/stone warmth), OR on any dark-mode Hero / CTA section (`min-height >= 60dvh`) that does not already have a full-bleed WebGPU canvas or video background.
- **Suppress when**: Inside dense data tables, FAQ accordions, or directly behind long-form reading columns where background luminance variation reduces APCA local contrast below `|Lc| 90`.
- **Execution**:
  1. Anchor the `radial-gradient` in OKLCH space directly at the section's **visual mass centroid `(cx, cy)`** (`at var(--focal-x, 68%) var(--focal-y, 34%)`), never blindly at `50% 50%`.
  2. Dither the field with a zero-network inline `<svg><filter id="ui-grain"><feTurbulence type="fractalNoise" baseFrequency="0.82" numOctaves="3" stitchTiles="stitch"/></filter></svg>` pseudo-element (`opacity: 0.025..0.042`, `mix-blend-mode: overlay` in dark mode, `multiply` in light mode).

### 4. When and How to Use a WebGPU Shader

A WebGPU shader is an **interactive mathematical proof** of the subject's core concept.

- **Activate when**: `D >= 0.60` AND `(P >= 0.60 || E >= 0.35)` on the **Hero** or a single dedicated **Showcase (`media`)** section, where the brand's value proposition involves continuous fields, compute throughput, spatial architecture, optics, cryptography, acoustics, or physical material transformation.
- **Suppress when**: The subject is purely literary, legal, or quiet archival (`D < 0.40` or `W >= 0.80` with `P < 0.35`), or when authentic physical photography/video of a real place or craft product is stronger proof than synthetic math.
- **Match Shader Math to Meaning**:
  - *Unified Platform / Composable Infra* -> Signed Distance Field (`SDF`) smooth-minimum (`smin`) boolean morphing driven by pointer proximity.
  - *High-Throughput Compute / Financial Flow* -> `@compute` instanced flow field (`10k-100k` anisotropic streaks) aligned to vector curl-noise.
  - *Security / Cryptography* -> Ordered Bayer dithering / lattice refraction over a crisp geometric SDF polyhedron.
  - *Creative / Optical / Hardware* -> Physical index-of-refraction (`IOR`) chromatic dispersion + specular normal mapping, or Three.js `WebGPURenderer` PBR CAD model.

### 5. Photo & Video Engineering (Formats, Codecs, Optics & Performance)

When a project uses photography or video, amateur execution ruins both aesthetics and Core Web Vitals (`LCP` and `CLS`). Follow these hard engineering rules:

#### A. Image Format Selection Matrix

| Format | MIME / Extension | Exact Use Case & Encoding Spec |
| --- | --- | --- |
| **AVIF (Primary)** | `image/avif` (`.avif`) | **Default for all photography, hero imagery, and gradients.** Supports 10-bit/12-bit color depth (eliminates dark-mode banding) and hardware film-grain synthesis (`avifenc --min 18 --max 24 -a cq-level=20 -a tune=ssim`). ~50% smaller than WebP. |
| **WebP (Fallback / UI)** | `image/webp` (`.webp`) | **Second `<source>` in `<picture>`**, or primary format for crisp pixel-aligned UI screenshots (`cwebp -q 84 -m 6`, or `-lossless` for diagram captures). |
| **SVG (Vector Only)** | `image/svg+xml` (`.svg`) | **Logos, parametric schematics, and UI icons only.** Always inline interactive SVGs and apply `vector-effect="non-scaling-stroke"`. |
| **PNG / JPEG (Legacy)** | `.png` / `.jpg` | **Never use as primary hero delivery.** Keep only as the final `<img src="...">` fallback inside `<picture>` if legacy browser support is required. |

#### B. Video Format & Codec Hierarchy

Never drop a raw 40MB H.264 `.mp4` with an empty audio track into a landing page.

| Codec / Container | `<source type="...">` | Role in `<video>` Stack |
| --- | --- | --- |
| **AV1 (Primary)** | `video/mp4; codecs="av01.0.05M.08"` or `video/webm; codecs="av01.0.05M.08"` | **First `<source>`.** Best visual quality per byte (~40% smaller than H.264/VP9). Target `1.5-3.0 MB` for a 6-second 1080p hero loop (`-crf 30 -b:v 0 -an`). |
| **HEVC / H.265 (Apple/Safari)** | `video/mp4; codecs="hvc1"` | **Second `<source>`** (especially required for **transparent alpha video** on Safari/iOS via `.mp4`/`.mov` with `hvc1`). |
| **VP9 (WebM)** | `video/webm; codecs="vp9"` | **Alternative/Alpha `<source>`** for Chromium/Firefox (supports alpha channel transparency via `yuva420p`). |
| **H.264 (Universal Fallback)** | `video/mp4; codecs="avc1.4d401f"` | **Final `<source>`.** Always encode with `-movflags +faststart -pix_fmt yuv420p -an -crf 23` so playback starts on the first network chunk. |

#### C. Mandatory DOM, Optical & Lifecycle Rules for Photo/Video

1. **Zero Layout Shift (`CLS = 0.000`) & Fast LCP**:
   - Every `<img>` and `<video>` MUST have an explicit CSS `aspect-ratio` (e.g. `aspect-ratio: 16 / 9` or `4 / 3`) and `object-fit: cover`.
   - The **Hero image** MUST carry `fetchpriority="high" loading="eager" decoding="async"`.
   - Every **below-the-fold image** MUST carry `loading="lazy" decoding="async"`.
2. **Video Loop Discipline**:
   - Every background/showcase `<video>` MUST set `autoplay muted loop playsinline preload="metadata"` and provide an `.avif` `poster="..."` matching the first frame.
   - Strip the audio track (`-an` in `ffmpeg`) from all muted loops -- an empty audio stream wastes 15% bandwidth and can block autoplay policies.
   - Pause offscreen videos via `IntersectionObserver` (`video.pause()` when `!entry.isIntersecting`) and halt autoplay when `prefers-reduced-motion: reduce` is active.
3. **Optical Palette Integration & Scrims**:
   - Raw external photographs often clash with the page's OKLCH palette. Unify them optically using `filter: contrast(1.04) saturate(0.92)` and a `1px` inner border (`inset 0 0 0 1px oklch(1 0 0 / 0.08)`).
   - Never place typography directly over an un-scrimmed photograph or video. Either place media in its own solved 12-column grid cell (`ple-engine` default) or insert an OKLCH gradient scrim that guarantees **APCA `|Lc| >= 75`** against the brightest pixel of the media.
4. **Zero-Copy WebGPU Video Textures (`importExternalTexture`)**:
   - When combining video with WebGPU (e.g. optical glass refraction, displacement, or real-time color grading over a video), use `device.importExternalTexture({ source: videoElement })` inside WGSL (`var tex: texture_external;`). This samples decoded hardware video frames directly in GPU memory with **zero CPU-GPU texture upload overhead**.

### 6. When and How to Use Micro-Animations

- **Interactive Affordance Haptics (Always Active)**: Every clickable or focusable element (`button`, `a`, `input`, pricing/feature cards) must respond within **1 frame (`< 8ms`)** using compositor-only properties (`transform`, `opacity`, `filter`).
- **Optical Specular Border Shift on Cards**: Separate cards from the ground using a `1px` border at APCA `|Lc| ~ 14` with a top-edge inner specular highlight (`box-shadow: inset 0 1px 0 0 oklch(1 0 0 / 0.06)`) that brightens to `var(--accent)` and lifts `translate3d(0, -2px, 0)` over `150ms cubic-bezier(0.22, 1, 0.36, 1)`.
- **Directional Glyph Kinematics**: Translate **only the inline `<svg>` arrow** (`translate3d(3px, -2px, 0)`) on button/link `:hover` while the text label remains stationary.

### 7. When and How to Use Custom SVG Icons & Vector Schematics

- **Ban Noun-Cliché Decorative Icons**: Never put a lightning bolt above "Fast", a shield above "Secure", or a rocket above "Scale". If an icon only repeats the heading noun as a cartoon pictogram, **delete it**.
- **Use Custom Two-Tone Parametric SVG Schematics**: Top feature cards or technical proofs with a custom geometric SVG micro-schematic (`viewBox="0 0 120 48"` or `28x28`) that diagrams the actual mechanism (latency timeline, consensus topology, optical wave, or orthographic projection).
- **Synchronize Stroke & Join Physics**:
  - Match `stroke-width` (`1.25px` to `2.0px`) to the active font weight and set `vector-effect="non-scaling-stroke"`.
  - Match `stroke-linecap`/`stroke-linejoin` to `radius_px` (`square`/`miter` when `radius_px <= 5`, `round` when `radius_px > 5`).
  - Use **two optical layers** inside every SVG: muted structural reference lines (`stroke-opacity="0.25"`) + full-contrast active focal vector (`stroke="var(--accent)"` or `opacity="1.0"`).

## References

- Read `references/modality-calculus.md` for copy-ready CSS/SVG templates of noisy OKLCH gradients, specular card borders, and two-tone parametric SVG micro-schematics.
- Read `references/media-formats-and-uniqueness.md` for production `<picture>` (`AVIF`/`WebP`) and multi-codec `<video>` (`AV1`/`HEVC`/`VP9`/`H.264`) markup, `ffmpeg`/`avifenc` encoding commands, WebGPU `importExternalTexture` shaders, and domain-unique interactive artifact blueprints.

## Gotchas

- **Never stack two high-frequency backgrounds in the same container**: Do not put a CSS `feTurbulence` noise overlay on top of a WebGPU `<canvas>` or `<video>`.
- **Never use un-poster'd or un-sized `<video>`**: A `<video>` without `aspect-ratio` and `poster` flashes a blank box during initial load and causes a severe layout shift when metadata arrives.
