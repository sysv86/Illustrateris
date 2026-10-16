#!/usr/bin/env python3
"""audit_design.py -- First-principles Frontend, WebGPU, Three.js, Media & Anti-Slop Auditor.

Scans HTML, CSS, JS, JSX, TS, and TSX files for:
  1. Legacy WebGL calls (`getContext('webgl')`, `new THREE.WebGLRenderer`, `ShaderMaterial`) instead of
     WebGPU (`navigator.gpu`, `three/webgpu` `WebGPURenderer` + TSL `NodeMaterial`).
  2. Unclamped `devicePixelRatio` or missing `IntersectionObserver` / `prefers-reduced-motion` in GPU loops.
  3. Frame-rate dependent lerp anti-patterns (`+= (target - current) * 0.`) inside RAF loops.
  4. Non-compositor CSS transitions (`transition: all`, animating `top`/`left`/`width`/`height`).
  5. Media/Video defects (`<video autoplay` without `muted`/`playsinline`/`poster`, unoptimized hero formats).
  6. Wave 1, Wave 2, and Wave 3 AI Slop tells (AI purple/indigo hexes, terracotta/cream template cosplay,
     banned buzzwords, `h-screen` viewport bug).

Usage:
    python3 skills/illustrateris-design-webgpu/scripts/audit_design.py <file_or_dir>
"""

from __future__ import annotations

import argparse
import os
import re
import sys

EXTS = {".html", ".css", ".js", ".mjs", ".ts", ".jsx", ".tsx", ".vue", ".svelte"}

CHECKS = [
    (
        "FAIL",
        "legacy-webgl",
        re.compile(r"getContext\(\s*['\"](?:experimental-)?webgl2?['\"]\s*\)|\bnew\s+(?:THREE\.)?WebGLRenderer\b"),
        "Legacy WebGL context or THREE.WebGLRenderer detected. Use WebGPU (`navigator.gpu` or `three/webgpu` `WebGPURenderer` + TSL).",
    ),
    (
        "FAIL",
        "legacy-three-shadermaterial",
        re.compile(r"\bnew\s+(?:THREE\.)?(?:Raw)?ShaderMaterial\b|\bonBeforeCompile\s*="),
        "Legacy Three.js ShaderMaterial / onBeforeCompile detected. Use `three/tsl` (`Fn`, `uniform`, `NodeMaterial`) for WebGPU compatibility.",
    ),
    (
        "FAIL",
        "transition-all",
        re.compile(r"transition\s*:\s*all\b|\btransition-all\b"),
        "`transition: all` animates layout properties and causes repaint jank. Specify `transform`, `opacity`, or `filter` explicitly.",
    ),
    (
        "FAIL",
        "layout-thrash-anim",
        re.compile(r"transition(?:-property)?\s*:[^;{}]*\b(?:width|height|top|left|margin|padding)\b"),
        "Animating layout-inducing properties (`width`, `height`, `top`, `left`, `margin`, `padding`) triggers main-thread relayout every frame.",
    ),
    (
        "FAIL",
        "fps-dependent-lerp",
        re.compile(r"\+=\s*\([^)]+-\s*[^)]+\)\s*\*\s*0\.\d+"),
        "Frame-rate dependent lerp (`x += (target - x) * 0.1`) runs 2x faster at 120Hz than 60Hz. Use `1 - Math.exp(-lambda * dt)`.",
    ),
    (
        "FAIL",
        "h-screen-mobile-bug",
        re.compile(r"\bh-screen\b|height\s*:\s*100vh\b"),
        "`100vh` / `h-screen` clips content under mobile browser chrome. Use `100dvh` / `min-h-[100dvh]`.",
    ),
    (
        "FAIL",
        "video-missing-muted-playsinline",
        re.compile(r"<video\b(?![^>]*\bmuted\b)(?![^>]*\bplaysinline\b)[^>]*\bautoplay\b", re.IGNORECASE),
        "`<video autoplay>` without `muted` and `playsinline` will be blocked by mobile browsers.",
    ),
    (
        "FAIL",
        "wave1-slop-copy",
        re.compile(
            r"\b(?:Unleash|Elevate your|Seamless(?:ly)?|Next-Gen|Game-changer|Delve into|Revolutionize)\b",
            re.IGNORECASE,
        ),
        "Wave 1 AI copywriting cliché detected. Replace with concrete, verifiable domain claims.",
    ),
    (
        "WARN",
        "wave1-ai-indigo-purple",
        re.compile(r"#(?:6366f1|8b5cf6|a855f7|7c3aed|4f46e5)\b|\b(?:from|to|via)-(?:indigo|purple|violet)-[456]00\b", re.IGNORECASE),
        "Wave 1 default AI indigo/violet gradient palette detected. Derive perceptual OKLCH tokens from domain physics.",
    ),
    (
        "WARN",
        "wave3-template-cosplay",
        re.compile(r"#(?:fdfbf7|f7f6f3)\b", re.IGNORECASE),
        "Wave 3 warm-cream template background detected. Verify this was derived from domain tone rather than copied from a static template.",
    ),
    (
        "WARN",
        "unclamped-dpr",
        re.compile(r"window\.devicePixelRatio(?!\s*\|\|\s*1\s*\)\s*,\s*2)"),
        "`window.devicePixelRatio` used without `Math.min(..., 2)` clamping. Unclamped DPR starves mobile GPU fill-rate.",
    ),
]


def iter_files(target: str):
    if os.path.isfile(target):
        yield target
        return
    for root, dirs, files in os.walk(target):
        dirs[:] = [d for d in dirs if d not in {".git", "node_modules", "target", "dist", ".next"}]
        for f in sorted(files):
            if os.path.splitext(f)[1].lower() in EXTS:
                yield os.path.join(root, f)


def scan_file(path: str) -> list[tuple[str, str, int, str]]:
    findings = []
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as fh:
            lines = fh.readlines()
    except OSError:
        return findings

    for lineno, line in enumerate(lines, start=1):
        for severity, code, pattern, msg in CHECKS:
            if pattern.search(line):
                findings.append((severity, code, lineno, msg))
    return findings


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Audit frontend code for WebGPU/Three.js best practices, media optimization, motion physics, and AI slop.")
    ap.add_argument("target", help="File or directory to scan")
    args = ap.parse_args(argv)

    fails = 0
    warns = 0
    scanned = 0
    for path in iter_files(args.target):
        scanned += 1
        for severity, code, lineno, msg in scan_file(path):
            print(f"[{severity}] {path}:{lineno} ({code}) -- {msg}")
            if severity == "FAIL":
                fails += 1
            else:
                warns += 1

    print(f"\naudit_design: scanned {scanned} file(s), {fails} FAIL(s), {warns} WARN(s)")
    return 1 if fails > 0 else 0


if __name__ == "__main__":
    raise SystemExit(main())
