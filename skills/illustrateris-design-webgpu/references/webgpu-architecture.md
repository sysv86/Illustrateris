# WebGPU Architecture & Production Boilerplate for Frontends

## 1. Complete Lifecycle-Safe WebGPU Controller

Every WebGPU surface in a landing page or frontend application must obey four non-negotiable hardware constraints:

1. **Single Shared `GPUDevice`**: Never instantiate multiple `GPUDevice` objects on the same page.
2. **DPR Clamping (`<= 2.0`)**: Clamping `devicePixelRatio` prevents 4x-9x fill-rate starvation on mobile OLED and 5K displays.
3. **Zero Idle GPU Drain**: Pause the `requestAnimationFrame` loop via `IntersectionObserver` when the canvas scrolls out of view and via `visibilitychange` when the tab is backgrounded.
4. **Graceful Static Fallback**: Render a single static frame when `prefers-reduced-motion: reduce` is active, and fall back to CSS when `navigator.gpu` is unavailable.

### Production TypeScript / JavaScript Module

```javascript
export async function mountWebGpuSurface(canvas, wgslShaderCode) {
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (!('gpu' in navigator)) {
    canvas.dataset.gpuFallback = 'css';
    return null;
  }

  const adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
  if (!adapter) {
    canvas.dataset.gpuFallback = 'css';
    return null;
  }

  const device = await adapter.requestDevice();
  const context = canvas.getContext('webgpu');
  const format = navigator.gpu.getPreferredCanvasFormat();
  context.configure({ device, format, alphaMode: 'premultiplied' });

  // Uniform buffer: [time, dt, width, height, pointerX, pointerY, scrollY, aspect] (32 bytes, 16-byte aligned)
  const uniformData = new Float32Array(8);
  const uniformBuffer = device.createBuffer({
    size: uniformData.byteLength,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const shaderModule = device.createShaderModule({ code: wgslShaderCode });
  const pipeline = await device.createRenderPipelineAsync({
    layout: 'auto',
    vertex: { module: shaderModule, entryPoint: 'vs_main' },
    fragment: {
      module: shaderModule,
      entryPoint: 'fs_main',
      targets: [{ format }],
    },
    primitive: { topology: 'triangle-list' },
  });

  const bindGroup = device.createBindGroup({
    layout: pipeline.getBindGroupLayout(0),
    entries: [{ binding: 0, resource: { buffer: uniformBuffer } }],
  });

  let width = 1;
  let height = 1;
  const resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const dpr = Math.min(window.devicePixelRatio || 1, 2.0);
      width = Math.max(1, Math.floor(entry.contentRect.width * dpr));
      height = Math.max(1, Math.floor(entry.contentRect.height * dpr));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
    }
  });
  resizeObserver.observe(canvas);

  // Frame-rate independent pointer smoothing
  let targetPx = 0.5, targetPy = 0.5;
  let curPx = 0.5, curPy = 0.5;
  window.addEventListener('pointermove', (e) => {
    const rect = canvas.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      targetPx = (e.clientX - rect.left) / rect.width;
      targetPy = (e.clientY - rect.top) / rect.height;
    }
  }, { passive: true });

  let rafId = 0;
  let running = false;
  let lastTime = performance.now();

  function renderFrame(now) {
    const dt = Math.min(0.1, Math.max(0.0001, (now - lastTime) * 0.001));
    lastTime = now;

    // Frame-rate independent exponential decay (lambda = 10.0)
    const alpha = 1.0 - Math.exp(-10.0 * dt);
    curPx += (targetPx - curPx) * alpha;
    curPy += (targetPy - curPy) * alpha;

    uniformData[0] = now * 0.001;
    uniformData[1] = dt;
    uniformData[2] = width;
    uniformData[3] = height;
    uniformData[4] = curPx;
    uniformData[5] = curPy;
    uniformData[6] = window.scrollY;
    uniformData[7] = width / Math.max(1, height);
    device.queue.writeBuffer(uniformBuffer, 0, uniformData);

    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [{
        view: context.getCurrentTexture().createView(),
        clearValue: { r: 0, g: 0, b: 0, a: 0 },
        loadOp: 'clear',
        storeOp: 'store',
      }],
    });
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.draw(3, 1, 0, 0); // Fullscreen oversized triangle (zero vertex buffer overhead)
    pass.end();
    device.queue.submit([encoder.finish()]);

    if (running && !reducedMotion) {
      rafId = requestAnimationFrame(renderFrame);
    }
  }

  const intersectionObserver = new IntersectionObserver((entries) => {
    const visible = entries.some((e) => e.isIntersecting);
    if (visible && !running) {
      running = true;
      lastTime = performance.now();
      rafId = requestAnimationFrame(renderFrame);
    } else if (!visible && running) {
      running = false;
      cancelAnimationFrame(rafId);
    }
  });
  intersectionObserver.observe(canvas);

  document.addEventListener('visibilitychange', () => {
    if (document.hidden && running) {
      running = false;
      cancelAnimationFrame(rafId);
    } else if (!document.hidden && !running) {
      running = true;
      lastTime = performance.now();
      rafId = requestAnimationFrame(renderFrame);
    }
  });

  device.lost.then(() => {
    running = false;
    cancelAnimationFrame(rafId);
    canvas.dataset.gpuFallback = 'lost';
  });

  return {
    destroy() {
      running = false;
      cancelAnimationFrame(rafId);
      resizeObserver.disconnect();
      intersectionObserver.disconnect();
      uniformBuffer.destroy();
    },
  };
}
```

## 2. Fullscreen Triangle + SDF Raymarching WGSL Template

Using a single fullscreen oversized triangle (`pass.draw(3)`) avoids vertex buffer allocation and eliminates the diagonal quad seam artifact along the screen center:

```wgsl
struct Uniforms {
  time: f32,
  dt: f32,
  resolution: vec2<f32>,
  pointer: vec2<f32>,
  scroll_y: f32,
  aspect: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
  var pos = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 3.0, -1.0),
    vec2<f32>(-1.0,  3.0)
  );
  var out: VsOut;
  let p = pos[vid];
  out.pos = vec4<f32>(p, 0.0, 1.0);
  out.uv = p * 0.5 + 0.5;
  return out;
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
  let h = max(k - abs(a - b), 0.0) / k;
  return min(a, b) - h * h * k * 0.25;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let p = (in.uv - 0.5) * vec2<f32>(u.aspect, 1.0);
  let cursor = (u.pointer - vec2<f32>(0.5, 0.5)) * vec2<f32>(u.aspect, -1.0);
  let d1 = length(p) - 0.24;
  let d2 = length(p - cursor) - 0.14;
  let d = smin(d1, d2, 0.18);
  let edge = smoothstep(0.004, -0.004, d);
  let col = mix(vec3<f32>(0.07, 0.08, 0.10), vec3<f32>(0.88, 0.91, 0.96), edge);
  return vec4<f32>(col, 1.0);
}
```
