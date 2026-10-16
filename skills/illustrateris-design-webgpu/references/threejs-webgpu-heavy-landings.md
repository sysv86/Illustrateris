# Three.js (`three/webgpu` + TSL) Tutorial for Heavy 3D Landing Pages

When a flagship landing page requires **complex 3D product/CAD models (`.glb`), PBR materials (clearcoat, transmission, dispersion), spatial camera scroll choreography, or 50,000+ instanced 3D meshes**, use **Three.js with `WebGPURenderer` and TSL (`Three.js Shading Language`)**.

## 1. Why `three/webgpu` + TSL Replaces Legacy Three.js (`WebGLRenderer`)

- **Legacy Three.js (`WebGLRenderer` + `ShaderMaterial` / `onBeforeCompile`)**: Concatenates raw GLSL strings at runtime, blocks the main thread during shader compilation, cannot run `@compute` storage buffers, and suffers from WebGL state-machine CPU overhead on multi-object scenes.
- **Modern Three.js (`three/webgpu` + `three/tsl`)**:
  1. Uses `WebGPURenderer`, which compiles node graphs into immutable **WGSL** `GPURenderPipeline` and `GPUComputePipeline` objects (and automatically falls back if WebGPU is unavailable).
  2. Replaces raw GLSL strings and fragile `onBeforeCompile` hooks with composable, type-safe **TSL functions** (`Fn`, `uniform`, `storage`, `positionLocal`, `normalWorld`, `uv`, `time`).
  3. Allows `renderer.compute(computeNode)` to update `StorageInstancedBufferAttribute` directly in GPU VRAM with zero CPU loop overhead.

## 2. Complete Production Setup: `WebGPURenderer` + Meshopt/KTX2 + Scroll Camera Rig

```javascript
import * as THREE from 'three/webgpu';
import {
  Fn, uniform, uv, time, positionLocal, normalWorld,
  sin, cos, mix, smoothstep, vec3, float
} from 'three/tsl';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { KTX2Loader } from 'three/addons/loaders/KTX2Loader.js';
import { MeshoptDecoder } from 'three/addons/libs/meshopt_decoder.module.js';

export async function mountHeavy3DLanding(canvas, modelUrl) {
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // 1. Initialize WebGPURenderer with clamped DPR (<= 2.0)
  const renderer = new THREE.WebGPURenderer({
    canvas,
    antialias: true,
    alpha: true,
    powerPreference: 'high-performance',
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2.0));
  renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;
  await renderer.init();

  // 2. Scene, Camera & Spline Scroll Path
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(
    38,
    canvas.clientWidth / Math.max(1, canvas.clientHeight),
    0.1,
    100
  );

  // Camera travels along a smooth 3D Catmull-Rom spline as the user scrolls
  const cameraSpline = new THREE.CatmullRomCurve3([
    new THREE.Vector3(0.0, 0.8, 4.2),
    new THREE.Vector3(1.6, 0.4, 2.8),
    new THREE.Vector3(-1.2, -0.3, 2.4),
    new THREE.Vector3(0.0, 0.2, 1.8),
  ]);
  const lookAtTarget = new THREE.Vector3(0, 0, 0);

  // 3. Studio Lighting (Rim + Key + Fill)
  const keyLight = new THREE.DirectionalLight(0xffffff, 2.8);
  keyLight.position.set(3, 5, 4);
  const rimLight = new THREE.DirectionalLight(0x88aaff, 1.6);
  rimLight.position.set(-4, 2, -3);
  const ambientLight = new THREE.AmbientLight(0xffffff, 0.4);
  scene.add(keyLight, rimLight, ambientLight);

  // 4. Custom TSL Node Material (Procedural Anisotropic / Pulse Shader in WGSL)
  const pointerUniform = uniform(new THREE.Vector2(0.5, 0.5));
  const scrollUniform = uniform(0.0);

  const customMaterial = new THREE.MeshPhysicalNodeMaterial({
    roughness: 0.18,
    metalness: 0.85,
    clearcoat: 0.6,
    clearcoatRoughness: 0.1,
  });

  // TSL Vertex displacement node (compiles to WGSL @vertex)
  customMaterial.positionNode = Fn(() => {
    const pos = positionLocal.toVar();
    const wave = sin(pos.y.mul(6.0).add(time.mul(2.0))).mul(0.015).mul(scrollUniform);
    pos.x.addAssign(normalWorld.x.mul(wave));
    return pos;
  })();

  // TSL Emissive rim-light node (compiles to WGSL @fragment)
  customMaterial.emissiveNode = Fn(() => {
    const fresnel = float(1.0).sub(normalWorld.z.abs()).pow(3.0);
    const baseRim = vec3(0.18, 0.42, 0.95);
    return baseRim.mul(fresnel).mul(0.65);
  })();

  // 5. Load Compressed GLB (Meshopt Geometry + KTX2 GPU-Compressed VRAM Textures)
  const ktx2Loader = new KTX2Loader()
    .setTranscoderPath('/basis/')
    .detectSupport(renderer);

  const gltfLoader = new GLTFLoader();
  gltfLoader.setMeshoptDecoder(MeshoptDecoder);
  gltfLoader.setKTX2Loader(ktx2Loader);

  let rootModel = null;
  if (modelUrl) {
    const gltf = await gltfLoader.loadAsync(modelUrl);
    rootModel = gltf.scene;
    scene.add(rootModel);
  } else {
    // Fallback procedural CAD torus-knot if no external .glb is passed
    const geo = new THREE.TorusKnotGeometry(0.75, 0.22, 160, 32);
    rootModel = new THREE.Mesh(geo, customMaterial);
    scene.add(rootModel);
  }

  // 6. Responsive ResizeObserver
  const resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const w = Math.max(1, entry.contentRect.width);
      const h = Math.max(1, entry.contentRect.height);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h, false);
    }
  });
  resizeObserver.observe(canvas);

  // 7. Frame-Rate Independent Scroll & Pointer Choreography
  let targetScroll = 0;
  let currentScroll = 0;
  let targetPtrX = 0, targetPtrY = 0;
  let curPtrX = 0, curPtrY = 0;

  const onScroll = () => {
    const maxScroll = Math.max(1, document.documentElement.scrollHeight - window.innerHeight);
    targetScroll = Math.min(1, Math.max(0, window.scrollY / maxScroll));
  };
  window.addEventListener('scroll', onScroll, { passive: true });
  onScroll();

  window.addEventListener('pointermove', (e) => {
    targetPtrX = (e.clientX / Math.max(1, window.innerWidth) - 0.5) * 2.0;
    targetPtrY = (e.clientY / Math.max(1, window.innerHeight) - 0.5) * 2.0;
  }, { passive: true });

  let rafId = 0;
  let running = false;
  let lastTime = performance.now();

  function animate(now) {
    const dt = Math.min(0.1, Math.max(0.0001, (now - lastTime) * 0.001));
    lastTime = now;

    // Frame-rate independent exponential damping (identical at 60Hz, 120Hz, 240Hz)
    const alphaScroll = 1.0 - Math.exp(-8.0 * dt);
    const alphaPtr = 1.0 - Math.exp(-10.0 * dt);
    currentScroll += (targetScroll - currentScroll) * alphaScroll;
    curPtrX += (targetPtrX - curPtrX) * alphaPtr;
    curPtrY += (targetPtrY - curPtrY) * alphaPtr;

    scrollUniform.value = currentScroll;
    pointerUniform.value.set(curPtrX, curPtrY);

    // Move camera along 3D spline + subtle parallax offset
    cameraSpline.getPointAt(currentScroll, camera.position);
    camera.position.x += curPtrX * 0.18;
    camera.position.y += -curPtrY * 0.12;
    camera.lookAt(lookAtTarget);

    if (rootModel) {
      rootModel.rotation.y = currentScroll * Math.PI * 1.5 + curPtrX * 0.25;
    }

    renderer.render(scene, camera);

    if (running && !reducedMotion) {
      rafId = requestAnimationFrame(animate);
    }
  }

  // 8. IntersectionObserver & Visibility Pause (Zero Offscreen GPU Drain)
  const io = new IntersectionObserver((entries) => {
    const vis = entries.some((e) => e.isIntersecting);
    if (vis && !running) {
      running = true;
      lastTime = performance.now();
      rafId = requestAnimationFrame(animate);
    } else if (!vis && running) {
      running = false;
      cancelAnimationFrame(rafId);
    }
  });
  io.observe(canvas);

  // 9. Clean VRAM Disposal
  return {
    destroy() {
      running = false;
      cancelAnimationFrame(rafId);
      window.removeEventListener('scroll', onScroll);
      resizeObserver.disconnect();
      io.disconnect();
      ktx2Loader.dispose();
      scene.traverse((obj) => {
        if (obj.geometry) obj.geometry.dispose();
        if (obj.material) {
          const mats = Array.isArray(obj.material) ? obj.material : [obj.material];
          for (const m of mats) m.dispose();
        }
      });
      renderer.dispose();
    },
  };
}
```

## 3. Asset Budget & Pre-Flight CLI Commands for Heavy 3D Landings

Never ship a `.glb` over `3.5 MB` for a hero scene. Run these two commands during asset preparation:

```bash
# 1. Compress geometry with Meshopt (-cc) and convert textures to KTX2 BasisU (-tc):
npx gltfpack -i raw-model.glb -o public/models/hero.opt.glb -cc -tc -ts 0.75

# 2. Inspect final draw calls, triangle count (< 150k triangles), and VRAM footprint:
npx gltf-transform inspect public/models/hero.opt.glb
```
