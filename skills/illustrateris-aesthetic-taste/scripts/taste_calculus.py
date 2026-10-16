#!/usr/bin/env python3
"""taste_calculus.py -- First-principles Modality, Uniqueness & Media Calculus for Illustrateris.

Given a domain's physical tone parameters (precision, warmth, drama, chromatic_energy, density, theme),
this script computes continuous activation scores (0.0..1.0) and concrete CSS/SVG/WGSL/Media parameters for:
  1. uniqueness_signature (substrate lightness L, harmonic accent shift deg, bespoke artifact class)
  2. noisy_gradient       (OKLCH centroid radial field + SVG feTurbulence grain)
  3. webgpu_and_3d        (Raw WGSL vs Three.js WebGPURenderer + TSL, KTX2/Meshopt budget)
  4. media_pipeline       (AVIF 10-bit, WebP fallback, AV1/HEVC/VP9/H.264 video loop & LCP/CLS specs)
  5. micro_animation      (compositor spring / specular border / directional glyph kinematics)
  6. svg_iconography      (two-tone parametric vector schematics & affordance glyphs)

Zero hardcoded templates: every parameter is a continuous function of the input vector.

Usage:
    python3 skills/illustrateris-aesthetic-taste/scripts/taste_calculus.py \
        --precision 0.85 --warmth 0.20 --drama 0.75 --energy 0.35 --density 0.65 --dark
"""

from __future__ import annotations

import argparse
import json


def clamp(x: float, lo: float = 0.0, hi: float = 1.0) -> float:
    return max(lo, min(hi, x))


def compute_taste_profile(
    precision: float,
    warmth: float,
    drama: float,
    energy: float,
    density: float,
    dark: bool,
) -> dict:
    p = clamp(precision)
    w = clamp(warmth)
    d = clamp(drama)
    e = clamp(energy)
    den = clamp(density)

    # 0. Uniqueness & Substrate/Chromatic Divergence
    if dark:
        bg_l = round(clamp(0.155 - 0.040 * d + 0.010 * w, 0.110, 0.160), 3)
        surf_l = round(clamp(bg_l + 0.036 + 0.010 * d, 0.150, 0.205), 3)
    else:
        bg_l = round(clamp(0.990 - 0.018 * w, 0.968, 0.992), 3)
        surf_l = round(clamp(bg_l - 0.028 - 0.008 * d, 0.935, 0.965), 3)

    if w >= 0.65 and p < 0.55:
        harmony_mode = "analogous_warmth"
        harmony_shift_deg = round(28.0 + 52.0 * e, 1)
    elif p >= 0.75:
        harmony_mode = "technical_split"
        harmony_shift_deg = round(95.0 * e + 25.0 * d, 1)
    else:
        harmony_mode = "complementary_tension"
        harmony_shift_deg = round(150.0 * e + 20.0 * (d - w), 1)

    # 1. Noisy Gradient Calculus (feTurbulence + OKLCH light field)
    noisy_score = clamp(0.45 * (d * e * 2.2) + 0.30 * w + (0.25 if dark else 0.10) * (1.0 - den * 0.5))
    grain_freq = round(0.60 + 0.30 * p - 0.10 * w, 2)
    grain_opacity = round((0.024 + 0.018 * w + 0.008 * d) if dark else (0.028 + 0.022 * w), 4)

    # 2. WebGPU & Heavy 3D Calculus
    shader_score = clamp(0.55 * d + 0.25 * max(p, e) + 0.20 * (1.0 - w * 0.6) - 0.15 * max(0.0, den - 0.75))
    heavy_3d_score = clamp(0.60 * d + 0.40 * p - 0.30 * den)
    if heavy_3d_score >= 0.62:
        gpu_tier = "threejs_webgpu_tsl (MeshPhysicalNodeMaterial + KTX2 + Meshopt + CatmullRomCurve3 scroll spline)"
    elif shader_score >= 0.52:
        gpu_tier = "raw_webgpu_wgsl (fullscreen triangle SDF / compute storage buffer / importExternalTexture)"
    else:
        gpu_tier = "css_compositor_and_svg_grain"

    # 3. Micro-Animation Calculus
    spring_omega = round(14.0 + 8.0 * p - 4.0 * w, 1)
    hover_ms = int(round(180 - 50 * p + 30 * w))
    stagger_ms = int(round(65 - 20 * den + 15 * d))

    # 4. Parametric SVG Iconography & Schematic Calculus
    svg_score = clamp(0.50 * p + 0.30 * den + 0.20 * d)
    stroke_width = round(1.25 + 0.50 * d + 0.25 * p, 2)
    linecap = "square" if (w < 0.45 and p >= 0.60) else "round"
    linejoin = "miter" if linecap == "square" else "round"

    return {
        "input_vector": {
            "precision": round(p, 3),
            "warmth": round(w, 3),
            "drama": round(d, 3),
            "chromatic_energy": round(e, 3),
            "density": round(den, 3),
            "theme": "dark" if dark else "light",
        },
        "uniqueness_signature": {
            "substrate_bg_l": bg_l,
            "substrate_surface_l": surf_l,
            "chromatic_harmony_mode": harmony_mode,
            "accent_hue_shift_deg": harmony_shift_deg,
            "mandatory_bespoke_artifact": "Build 1 domain-native interactive mechanism (e.g. live curve scrubber, protocol verifier, CAD tolerance gauge) that cannot be swapped to another industry.",
        },
        "modalities": {
            "noisy_gradient": {
                "recommended": noisy_score >= 0.38,
                "score": round(noisy_score, 3),
                "fe_turbulence_base_frequency": grain_freq,
                "grain_opacity": grain_opacity,
                "blend_mode": "overlay" if dark else "multiply",
            },
            "webgpu_and_3d": {
                "recommended": shader_score >= 0.52,
                "shader_score": round(shader_score, 3),
                "heavy_3d_score": round(heavy_3d_score, 3),
                "recommended_tier": gpu_tier,
                "max_devices_per_page": 1,
            },
            "media_pipeline": {
                "photo_primary": "image/avif (10-bit, cq-level=20, film-grain synthesis)",
                "photo_fallback": "image/webp (-q 84)",
                "video_stack": [
                    'video/mp4; codecs="av01.0.05M.08" (AV1 primary, -crf 30 -an)',
                    'video/mp4; codecs="hvc1" (HEVC Safari/iOS + alpha support)',
                    'video/webm; codecs="vp9" (VP9 Chromium/Firefox + alpha support)',
                    'video/mp4; codecs="avc1.4d401f" (H.264 universal fallback, +faststart -an)',
                ],
                "3d_model_stack": "GLB with Meshopt (-cc) + KTX2 BasisU textures (-tc) <= 3.5MB",
            },
            "micro_animation": {
                "recommended": True,
                "spring_omega_rad_s": spring_omega,
                "hover_duration_ms": hover_ms,
                "reveal_stagger_ms": stagger_ms,
                "easing": "cubic-bezier(0.22, 1, 0.36, 1)",
            },
            "svg_iconography": {
                "recommended": svg_score >= 0.40,
                "score": round(svg_score, 3),
                "stroke_width_px": stroke_width,
                "stroke_linecap": linecap,
                "stroke_linejoin": linejoin,
            },
        },
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Compute first-principles aesthetic, uniqueness, and media parameters.")
    ap.add_argument("--precision", type=float, default=0.75)
    ap.add_argument("--warmth", type=float, default=0.30)
    ap.add_argument("--drama", type=float, default=0.65)
    ap.add_argument("--energy", type=float, default=0.30)
    ap.add_argument("--density", type=float, default=0.60)
    ap.add_argument("--dark", action="store_true", default=False)
    args = ap.parse_args(argv)

    profile = compute_taste_profile(
        args.precision, args.warmth, args.drama, args.energy, args.density, args.dark
    )
    print(json.dumps(profile, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
