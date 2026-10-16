<div align="center">
  <table>
    <tr>
      <td valign="middle">
        <img src="readme/brand/Illustrateris-banner.svg" alt="Illustrateris — procedural layout engine & design skills" width="400">
      </td>
      <td valign="middle">
        <img src="readme/brand/illustrateris-logo.svg" alt="Illustrateris logo" width="150" height="150">
      </td>
    </tr>
  </table>
</div>

# Illustrateris (`ple-engine` + Agent Skills)

First-principles frontend engineering: a deterministic Rust **Procedural Layout Engine (`ple-engine`)** paired with four modular **Agent Skills** (`skills/`) for De-PLE ejection, Aesthetic Taste & Modality Calculus, WebGPU shader architecture, and universal zero-hardcode skill conflict isolation.

Release history: [CHANGELOG.md](CHANGELOG.md). Verification harness: [verify/README.md](verify/README.md).

## Agent Skills (`skills/`)

Install all skills via the Agent Skills CLI:

```sh
npx skills add https://github.com/sysv86/Illustrateris
```

| Skill Folder | Install Name | Responsibility |
|---|---|---|
| [`skills/illustrateris-ple-engine`](skills/illustrateris-ple-engine/SKILL.md) | `illustrateris-ple-engine` | Remove/eject `ple-engine` (`eject_ple.py` / `ple eject`) into clean `:root` CSS Custom Properties and semantic `.ui-*` classes, operate in Zero-Binary mathematical mode, or compile Context JSON layouts. |
| [`skills/illustrateris-aesthetic-taste`](skills/illustrateris-aesthetic-taste/SKILL.md) | `illustrateris-aesthetic-taste` | Continuous Aesthetic & Modality Calculus (`taste_calculus.py`): teaches the LLM when and how to deploy **noisy gradients** (`feTurbulence` + OKLCH light fields), **WebGPU shaders**, **micro-animations**, and **parametric two-tone SVG schematics** without hardcoded templates. |
| [`skills/illustrateris-design-webgpu`](skills/illustrateris-design-webgpu/SKILL.md) | `illustrateris-design-webgpu` | Why WebGPU replaces WebGL, when to deploy WebGPU in frontends/landings (1-device budget, DPR clamping, `IntersectionObserver` pause), first-principles design research, the **mobile-first contract** (no desktop grid without a mobile fallback, breakable long tokens, wrapping navs, measure at 1440/768/375 before claiming done), delta-time spring choreography, and 3-wave anti-slop auditing (`audit_design.py`). |
| [`skills/illustrateris-skill-guard`](skills/illustrateris-skill-guard/SKILL.md) | `illustrateris-skill-guard` | Universal, zero-hardcode behavioral conflict firewall (`guard_skills.py`) that detects and suppresses any competing template/design skill by structural fingerprinting (immune to skill renaming). |

## Examples

`examples/` holds Context JSON documents; each one compiles to a standalone page.

| Example | What it is |
|---|---|
| [`examples/webgpu-flagship.json`](examples/webgpu-flagship.json) | Real-time graphics landing page with `<canvas>` shader surfaces. |
| [`examples/product-platform.json`](examples/product-platform.json) | The heaviest example (pricing, testimonial, FAQ, media); measured by the browser geometry test. |
| [`examples/fintech.json`](examples/fintech.json), [`examples/editorial.json`](examples/editorial.json), [`examples/playful.json`](examples/playful.json) | Compact cards for the three font personalities. |

To compile one and check its geometry at three widths:

```sh
cargo run --release -- compile examples/webgpu-flagship.json -o flagship.html
python3 verify/browser_verify.py --html flagship.html --viewport 1440,768,375
```

Every example compiles with `report.palette.all_checks_passed == true` and measures with
`horizontal_overflow == false`, every section contained and `overflowing_elements == []` at
1440, 768 and 375 px.
