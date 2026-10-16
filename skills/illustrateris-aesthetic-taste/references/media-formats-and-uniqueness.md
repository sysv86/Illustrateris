# Photo/Video Engineering, Codecs & Uniqueness Synthesis Reference

## 1. Production `<picture>` Markup (AVIF 10-Bit + WebP + LCP/CLS Lock)

### Hero Image (Above the Fold -- Maximize LCP, Zero CLS)

```html
<figure class="ui-media-frame">
  <picture>
    <source
      srcset="/assets/hero-768.avif 768w, /assets/hero-1440.avif 1440w, /assets/hero-2160.avif 2160w"
      sizes="(min-width: 1024px) 58vw, 100vw"
      type="image/avif"
    >
    <source
      srcset="/assets/hero-768.webp 768w, /assets/hero-1440.webp 1440w, /assets/hero-2160.webp 2160w"
      sizes="(min-width: 1024px) 58vw, 100vw"
      type="image/webp"
    >
    <img
      src="/assets/hero-1440.webp"
      alt="Orthographic cross-section of the optical compute module"
      width="1440"
      height="1080"
      fetchpriority="high"
      loading="eager"
      decoding="async"
      class="ui-media-img"
    >
  </picture>
</figure>
```

### Optical Grading & Zero-CLS CSS

```css
.ui-media-frame {
  position: relative;
  margin: 0;
  width: 100%;
  overflow: hidden;
  border-radius: var(--radius, 6px);
  background-color: var(--surface);
  box-shadow: inset 0 0 0 1px var(--border);
}

.ui-media-img,
.ui-media-video {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 4 / 3;
  object-fit: cover;
  filter: contrast(1.03) saturate(0.94);
}
```

## 2. Production `<video>` Stack (AV1 + HEVC + VP9 + H.264 + Offscreen Pause)

```html
<figure class="ui-media-frame">
  <video
    class="ui-media-video"
    autoplay
    muted
    loop
    playsinline
    preload="metadata"
    poster="/assets/reel-poster.avif"
    aria-label="Real-time telemetry capture of sub-millisecond state finality"
    data-auto-pause
  >
    <!-- 1. AV1 MP4: smallest size, hardware-decoded on M3+/RTX/modern Android -->
    <source src="/assets/reel-av1.mp4" type='video/mp4; codecs="av01.0.05M.08"'>
    <!-- 2. HEVC / H.265: hardware-decoded on Safari / iOS / macOS -->
    <source src="/assets/reel-hevc.mp4" type='video/mp4; codecs="hvc1"'>
    <!-- 3. VP9 WebM: Chromium / Firefox fallback -->
    <source src="/assets/reel-vp9.webm" type='video/webm; codecs="vp9"'>
    <!-- 4. H.264 MP4: universal fallback -->
    <source src="/assets/reel-h264.mp4" type='video/mp4; codecs="avc1.4d401f"'>
  </video>
</figure>
```

### Lifecycle Observer for `<video data-auto-pause>`

```javascript
export function mountVideoLifecycle() {
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const videos = document.querySelectorAll('video[data-auto-pause]');
  if (reducedMotion) {
    videos.forEach((v) => v.pause());
    return;
  }
  const io = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      const video = entry.target;
      if (entry.isIntersecting) {
        video.play().catch(() => {});
      } else {
        video.pause();
      }
    }
  }, { threshold: 0.1 });
  videos.forEach((v) => io.observe(v));
}
```

## 3. CLI Encoding Recipes (`avifenc` & `ffmpeg`)

```bash
# 10-bit AVIF with synthesized film grain (prevents dark-mode gradient banding):
avifenc --depth 10 --yuv 420 --min 18 --max 24 -a cq-level=20 -a tune=ssim input.png hero.avif

# AV1 MP4 (muted hero loop, faststart, zero audio stream):
ffmpeg -i input.mov -c:v libsvtav1 -crf 30 -preset 5 -pix_fmt yuv420p10le -an -movflags +faststart reel-av1.mp4

# H.264 Universal Fallback:
ffmpeg -i input.mov -c:v libx264 -crf 23 -preset slow -profile:v high -pix_fmt yuv420p -an -movflags +faststart reel-h264.mp4
```

## 4. Zero-Copy WebGPU Video Shader (`importExternalTexture`)

When applying optical refraction, displacement, or custom dithering to a `<video>`, never copy frames via `ctx2d.drawImage`. Bind the hardware decoder output directly in WGSL:

```javascript
// Inside requestAnimationFrame loop while video.readyState >= 2:
const externalTexture = device.importExternalTexture({ source: videoElement });
const bindGroup = device.createBindGroup({
  layout: pipeline.getBindGroupLayout(0),
  entries: [
    { binding: 0, resource: sampler },
    { binding: 1, resource: externalTexture },
    { binding: 2, resource: { buffer: uniformBuffer } },
  ],
});
```

```wgsl
@group(0) @binding(0) var videoSampler: sampler;
@group(0) @binding(1) var videoTex: texture_external;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let color = textureSampleBaseClampToEdge(videoTex, videoSampler, uv);
  return color;
}
```
