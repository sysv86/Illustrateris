# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-10-16

First release published from the `sysv86/Illustrateris` repository.

### Added

- **Repository packaging.** MIT `LICENSE`, GitHub Actions CI (`build`, `clippy`, `MSRV`, font-metric
  generator reproducibility, headless-browser geometry) and a tag-driven release workflow that
  publishes `ple` binaries for Linux, macOS and Windows.
- **Missing example.** `examples/webgpu-flagship.json`, referenced by the test suite and the
  `illustrateris-ple-engine` skill, was absent from the previous snapshot and is now included.

### Fixed

- **Clean/eject test false positive.** The assertion rejected any `ple-` substring, which also
  matched the `-apple-system` family in the font stack; it now checks for the real `ple-engine`
  generator meta/comments and `class="ple-…"` content-addressed classes.
- **Browser geometry comparison.** The harness now reports `has_element_children`; action links that
  wrap an inline icon are no longer compared against text-only width estimates (worst relative error
  back to ~5%, over 183 text runs).
- **Clippy is clean under `-D warnings`** (derived `Default`, slice parameters, iterator loops) and
  the declared MSRV `1.70` is verified to build and test.
- **`fonts/SOURCES.md`** now records the correct `metrics_data.rs` checksum; the generator reproduces
  the file byte for byte.

## [0.2.0] - 2026-10-15

### Added

- **Agent Skills (`skills/`).** Added three modular Agent Skills compatible with `npx skills add`, Claude Code, and Codex:
  - `illustrateris-ple-engine` (`skills/ple-engine/`): De-PLE ejection (`eject_ple.py` / `ple eject`), stripping `.ple-*` class hashes and generator metadata into `:root` CSS Custom Properties, Zero-Binary mathematical mode, and `ple-engine` lifecycle orchestration.
  - `illustrateris-design-webgpu` (`skills/design-foundations-webgpu/`): Architectural rules for WebGPU over WebGL, frontend/landing WebGPU deployment gates and lifecycle management (`ResizeObserver` DPR cap, `IntersectionObserver` pause), first-principles design research, frame-rate-independent animation physics, and 3-wave anti-slop auditing (`audit_design.py`).
  - `illustrateris-skill-guard` (`skills/skill-conflict-guard/`): Skill conflict firewall (`guard_skills.py`) that suppresses conflicting external template skills (`taste-skill`, `soft-skill`, `minimalist-skill`, `brutalist-skill`, `gpt-taste`, `stitch-skill`) and routes tasks deterministically across Illustrateris phases.
- **OpenType `GPOS` Kerning & Extended Unicode Metrics.** `tools/gen_font_metrics.py` now parses OpenType `GPOS` `kern` subtables (PairPos Format 1 & Format 2, plus LookupType 9 extensions) and extracts real per-glyph advances for 457 codepoints across ASCII, Latin-1, Latin Extended-A, Russian & Kazakh Cyrillic (`0x0400..0x04FF`), currencies (`₽`, `₸`, `€`, `₹`), and directional UI glyphs (`←`, `↑`, `→`, `↓`, `↗`, `↘`), plus East Asian Wide (`1.0em`) CJK fallback.
- **2D Visual Mass Centroid (`centroid_y`, `eccentricity_y`).** `src/layout/visual_mass.rs` and `src/layout/search.rs` now evaluate vertical centre of gravity alongside horizontal balance and include `centroid_y` and `eccentricity_y` in the JSON report.
- **WebGPU Canvas & Media Role (`Role::Media`, `VisualAnchor::WebGpuCanvas`).** Sections and heroes can now declare interactive WebGPU shader surfaces (`shader`, `aspect_ratio`) that participate in the BSP + Cassowary LP layout solver with area-derived visual mass and render accessible `<figure><canvas data-webgpu-shader="...">` elements.
- **Native De-PLE Ejection (`ple eject` / `--clean` & `tools/eject_ple.py`).** Emits `:root` CSS Custom Properties, clean `.ui-*` selectors, and zero `ple-engine` fingerprints.

## [0.1.0] - 2026-10-15

Initial release. Zero runtime dependencies; Rust 2021, built and tested with rustc 1.97.

### Added

- **Pipeline.** Context JSON → semantic profiler → OKLCH / APCA palette → BSP spatial grammar →
  Cassowary-style LP solver with candidate search → typed layout IR → semantic HTML.
- **Ten section roles.** `nav`, `hero`, `proof_matrix`, `feature_list`, `narrative`, `pricing`,
  `testimonial`, `faq`, `cta`, `footer`.
- **Real font metrics.** `src/text/metrics_data.rs` generated from bundled reference faces.
- **Two styling backends.** `--backend tailwind` and `--backend css`.
- **Browser verification.** `verify/browser_verify.py` / `verify/run.sh`.

[0.4.0]: https://github.com/sysv86/Illustrateris/releases/tag/v0.4.0
[0.2.0]: https://github.com/sysv86/Illustrateris/releases/tag/v0.2.0
[0.1.0]: https://github.com/sysv86/Illustrateris/releases/tag/v0.1.0
