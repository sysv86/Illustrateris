---
name: illustrateris-design-webgpu
description: Core frontend design engineering foundations for Illustrateris. Explains why to use WebGPU instead of WebGL, when WebGPU belongs in a frontend or landing page (and when it does not), when and how to use Three.js (three/webgpu + TSL) with KTX2/Meshopt assets for heavy 3D landing pages, how to research contemporary design from first principles without copying templates, how to engineer frame-rate-independent animations and scroll choreography, and how to eliminate all three waves of AI slop.
---

# Illustrateris Design Foundations: WebGPU, Three.js TSL, Choreography & Anti-Slop

Most AI design skills fail because they replace one statistical average with a rigid list of arbitrary bans and 20 static JSON templates. True design engineering comes from **physical constraints, optical math, and hardware-native execution**.

This skill defines the engineering and aesthetic laws for building modern frontends, interactive WebGPU surfaces, and heavy 3D landing pages.

## When to Use

- Designing or building a landing page, flagship marketing site, creative studio portfolio, hardware/compute showcase, or interactive web experience.
- Deciding between **Raw WebGPU (`navigator.gpu` + WGSL)**, **Three.js (`three/webgpu` + TSL)**, Canvas2D, or CSS for hero visuals and heavy 3D scenes.
- Researching visual direction and spatial composition for a new brand or product without falling back on stock SaaS templates.
- Implementing scroll choreography, spring physics, or micro-interactions.
- Auditing a frontend to eliminate Wave 1, Wave 2, and Wave 3 ("editorial template") AI slop.

## Steps

### 1. Why WebGPU Must Replace WebGL (Architectural Facts)

Never start new GPU-driven frontend work in raw WebGL1/WebGL2. WebGL is a 2011 JavaScript wrapper around OpenGL ES 2.0/3.0 built on a synchronous **global state machine** (`gl.bindBuffer`, `gl.activeTexture`, `gl.useProgram`).

| Dimension | WebGL (Legacy OpenGL ES) | WebGPU (`navigator.gpu` + WGSL) |
| --- | --- | --- |
| **State Model** | Global mutable state machine. Every draw call validates state combinations on the CPU driver thread, causing frame spikes. | Immutable **Pipeline State Objects** (`GPURenderPipeline`, `GPUComputePipeline`) compiled upfront via `createRenderPipelineAsync()`. |
| **Compute Shaders** | **None.** Physics, fluids, and particles require rendering to floating-point FBO textures ("ping-pong texture hacks"), saturating memory bandwidth. | **Native `@compute` shaders** (`@workgroup_size(64)`) with read/write `var<storage, read_write>` buffers shared directly with the vertex stage. |
| **Memory & CPU-GPU Sync** | Frequent CPU uniform uploads (`gl.uniform*`) and synchronous pipeline stalls on readback. | Explicit `GPUBindGroup` layouts and asynchronous `GPUCommandEncoder` submission. Zero per-frame CPU-GPU buffer copies. |
| **Shading Language** | GLSL strings concatenated at runtime with brittle preprocessor macros and driver-specific precision bugs (`mediump` vs `highp`). | **WGSL**: strictly typed, memory-safe, validated by the browser before reaching the OS graphics driver (Metal / Vulkan / DX12). |

### 2. When to Use WebGPU vs. Three.js (`three/webgpu` + TSL) in Frontends

Choose the exact rendering tier based on scene complexity:

| Rendering Tier | When to Use It | When NOT to Use It |
| --- | --- | --- |
| **Tier 1: CSS Compositor + SVG (`0 KB` JS GPU)** | Typography, 12-col grids, noisy OKLCH gradients (`feTurbulence`), specular card hover lifts, 2D parametric SVG schematics. | Continuous 3D fields, >1,000 particles, or physical refraction. |
| **Tier 2: Raw WebGPU (`navigator.gpu` + WGSL, `< 5 KB` JS)** | Fullscreen or hero **2D/2.5D SDF raymarching**, fluid/curl-noise compute particles, zero-copy video shaders (`importExternalTexture`), optical dispersion, custom dither/halftone passes. | Complex multi-mesh `.glb` CAD models, skeletal animations, or full PBR environment lighting with shadow maps. |
| **Tier 3: Three.js WebGPU (`three/webgpu` + `three/tsl`)** | **Heavy 3D landing pages**: loaded `.glb` product/hardware CAD models, PBR materials (`MeshPhysicalNodeMaterial` with clearcoat/transmission), 3D spline camera scroll rigs (`CatmullRomCurve3`), and `InstancedMesh` driven by TSL `storage()` compute buffers. | Simple 2D shader backgrounds where bundling a 150KB 3D scene graph is pure overhead. |

#### Critical Rules for Heavy 3D Landing Pages with Three.js (`three/webgpu`):

1. **Always Import `WebGPURenderer` and TSL (`three/webgpu` & `three/tsl`)**:
   - Never use legacy `new THREE.WebGLRenderer()` or legacy `ShaderMaterial` / `onBeforeCompile` GLSL strings.
   - Write custom vertex/fragment/compute logic in **TSL (`Fn`, `uniform`, `storage`, `positionLocal`, `normalWorld`, `uv`, `time`)** attached to `MeshPhysicalNodeMaterial` or `MeshStandardNodeMaterial`. TSL compiles natively to **WGSL** on WebGPU and falls back automatically if WebGPU is absent.
2. **Mandatory VRAM & Asset Compression (Meshopt + KTX2)**:
   - **Never load uncompressed `.gltf` or PNG/JPEG textures into a 3D scene.** A single 4096x4096 PNG decompresses to **67 MB of raw RGBA8 VRAM** (89 MB with mipmaps), crashing mobile Safari immediately.
   - Compress geometry with **Meshopt** (`gltfpack -i in.glb -o out.glb -cc -tc`) via `MeshoptDecoder`.
   - Compress textures into **KTX2 (`Basis Universal UASTC` for normals/roughness, `ETC1S` for diffuse)** via `KTX2Loader` so textures remain hardware-compressed (`BC7`/`ASTC`) directly inside GPU VRAM.
3. **Strict 1-Device Lifecycle & VRAM Disposal**:
   - Maximum **1 `WebGPURenderer` / `GPUDevice`** per page.
   - Clamp pixel ratio: `renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));`.
   - Pause the animation loop via `IntersectionObserver` when the canvas is offscreen and via `document.hidden`.
   - On component unmount / route change, traverse the scene and call `geometry.dispose()`, `material.dispose()`, `texture.dispose()`, and `renderer.dispose()`.

Read `references/threejs-webgpu-heavy-landings.md` for the complete production Three.js `WebGPURenderer` + TSL + KTX2 + Spline Camera Scroll Rig tutorial.

### 3. How to Research Contemporary Design (Without Template Cloning)

Never pick a design from a pre-made JSON deck. Before writing code, execute this 3-step structural research workflow:

1. **Extract Domain Materiality & Native Artifacts**:
   - List 5 physical or technical artifacts specific to the subject (e.g. for a settlement protocol: *merkle state trees, nanosecond timestamps, cryptographic hashes, order-book depth*; for an industrial studio: *CNC toolpaths, anodized aluminum tolerances, CAD orthographic projections*).
   - Build your bespoke interactive artifact, SVG schematics, and WebGPU shader directly from one of these native artifacts.
2. **Deconstruct Benchmark Sites Quantitatively (Not Cosmetically)**:
   - Extract **structural invariants**, never surface clichés:
     - **Scale Contrast**: Ratio between display headline and body copy (`>= 4.2x` to `8.5x`).
     - **Alignment Discipline**: Every element on the page snaps to 2 or 3 shared vertical alignment axes on a 12-column grid.
     - **Rhythm Alternation (Entropy Damping)**: Alternate between high-tension asymmetric sections (`8/4`, `7/5`, `9/3` splits) and calm structural sections. Never repeat the same section topology twice in a row.
     - **Chromatic Budget**: 85%+ neutral ground/surface (`oklch` low chroma), 10% primary ink (`|Lc| >= 90`), <= 5% high-chroma accent reserved strictly for the primary action or active focal node.
3. **Pass the Competitor Swap Test**:
   - Mentally swap the client's name and headline with a random competitor in another industry. If the layout and interactive canvas still fit, your design is generic.

### 4. How to Engineer Animations Properly

1. **The Compositor-Only Law**:
   - In CSS and Web Animations API, animate **only** `transform` (`translate3d`, `scale`, `rotate`), `opacity`, `filter`, and `clip-path`.
   - Never animate `top`, `left`, `width`, `height`, `margin`, `padding`, or `border-width`.
2. **Frame-Rate Independence (60Hz / 120Hz / 240Hz)**:
   - **BANNED**: `current += (target - current) * 0.1` inside `requestAnimationFrame`.
   - **REQUIRED**: Use delta-time exponential decay or critically damped springs:
     ```js
     const alpha = 1.0 - Math.exp(-lambda * dtSeconds);
     current += (target - current) * alpha;
     ```
3. **Temporal & Frequency Hierarchy**:
   - **Micro-feedback (press, hover, focus)**: `80ms-160ms`, `cubic-bezier(0.22, 1, 0.36, 1)`. Exit duration is ~70% of entrance duration.
   - **Structural transitions (drawers, modals, layout FLIP)**: `220ms-380ms`, critically damped spring (`zeta = 1.0`).
   - **Choreographed section reveals**: `500ms-800ms`, staggered `40ms-70ms` strictly in reading order, triggered **once** on scroll entry.
4. **Progressive Resilience**:
   - Gate initial hidden states behind `html.js-ready` and always honor `@media (prefers-reduced-motion: reduce)`.

### 5. How Not to Fall into AI Slop (All 3 Waves)

- **Wave 1 Slop (2023-2024 SaaS Cliché)**: Purple/indigo/violet neon gradients (`#6366f1` -> `#a855f7`), glowing blobs, heavy `backdrop-blur` glass cards, pill badge (`"✨ Introducing"`), 3 identical icon-in-box cards, and buzzwords (`"Unleash"`, `"Elevate"`, `"Seamless"`, `"Next-Gen"`).
- **Wave 2 Slop (2025 Bento & Eyebrow Spam)**: Uniform rounded Bento grids with fake grey-bar dashboards, tiny tracked uppercase eyebrows above every `<h2>`, one random italic/colored word in every H1, and uniform `fade-up` on every DOM node.
- **Wave 3 Slop (2025-2026 "Anti-Slop Template Cosplay")**: Yellow-cream `#FDFBF7` ground + terracotta/rust accent + oversized italic serif + random public-domain museum engravings copied from static template decks, or blindly banning monospace and 1px borders on technical products.

**Run the Automated Design, Media & Anti-Slop Auditor**:

```bash
python3 skills/illustrateris-design-webgpu/scripts/audit_design.py <path/to/project_or_html>
```

## References

- Read `references/webgpu-architecture.md` for raw WebGPU (`navigator.gpu` + WGSL) boilerplate and SDF/compute starter pipelines.
- Read `references/threejs-webgpu-heavy-landings.md` for the complete Three.js `WebGPURenderer` + TSL (`three/tsl`), KTX2/Meshopt asset loader, GPU compute instancing, and scroll-driven 3D spline camera rig tutorial.
- Read `references/motion-and-antislop.md` for delta-time spring equations and the Wave 1-3 slop detection matrix.

## Gotchas

- **Never use `h-screen` (`100vh`) on mobile heroes**: Always use `min-h-[100dvh]` (`100dvh`).
- **Never use legacy `ShaderMaterial` with `WebGPURenderer`**: It will fail to compile WGSL. Always use `NodeMaterial` (`MeshPhysicalNodeMaterial`, `SpriteNodeMaterial`) with `three/tsl`.
