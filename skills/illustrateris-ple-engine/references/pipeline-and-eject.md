# PLE Engine Pipeline & De-PLE Ejection Reference

## 1. Context JSON Schema (v0.2.0)

```json
{
  "intent": {
    "domain": "graphics_shader_infrastructure",
    "tone": "kinetic_spatial_precise",
    "density": 0.65,
    "asymmetry": 0.62,
    "chromatic_energy": 0.35,
    "theme": "dark",
    "hue": null,
    "seed": 42,
    "lang": "en",
    "title": "Optional Document Title"
  },
  "content_tree": [
    {
      "role": "hero",
      "id": "hero",
      "primary_claim": "Zero-Copy Compute Pipelines in the Browser",
      "supporting_text": "Execute 250,000-particle Verlet physics directly in WGSL storage buffers.",
      "action_type": "primary_button",
      "action_label": "Inspect Pipeline",
      "action_href": "#compute",
      "visual_anchor": "webgpu_canvas",
      "shader": "sdf_raymarch",
      "aspect_ratio": 1.333
    }
  ]
}
```

### Supported Section Roles (11)

- `nav`: Navigation bar (`primary_claim` wordmark + `items[{title, href}]` + optional action).
- `hero`: Full-viewport (`100dvh`) opening section (`primary_claim`, `supporting_text`, `action_type`, `visual_anchor`).
- `proof_matrix`: Quantitative metrics (`items[{metric, label}]`, rendered as `<dl>` with `tabular-nums`).
- `feature_list`: Asymmetric grid of capability blocks (`items[{title, body}]`).
- `narrative`: Long-form editorial claim + lead paragraph.
- `media` (`canvas` | `webgpu` | `showcase`): Interactive WebGPU / shader `<figure><canvas>` surfaces (`shader`, `aspect_ratio`, or `items[{title, body, shader, aspect_ratio}]`).
- `pricing`: Plan tiers (`items[{title, metric, label?, body?}]`).
- `testimonial`: Attributed quotes (`items[{body, title, label?}]` rendered as `<blockquote>` + `<cite>`).
- `faq`: Question-and-answer stack (`items[{title, body}]`).
- `cta`: Closing conversion block (`primary_claim`, `supporting_text`, `action_type`).
- `footer`: Closing links and colophon (`primary_claim`, `items[{title, href}]`, `supporting_text`).

## 2. De-PLE Ejection Transformations

When removing `ple-engine` (`ple eject` or `python3 skills/illustrateris-ple-engine/scripts/eject_ple.py`), three transformations occur:

1. **Metadata Removal**:
   - Strips `<meta name="generator" content="ple-engine ...">`
   - Strips `<!-- ple-engine ... -->`
2. **Token Extraction**:
   - Moves repeated inline `oklch(L C H)` values into `:root` custom properties (`--color-bg`, `--color-surface`, `--color-text`, `--color-border`, `--color-text-muted`, `--color-accent`, `--color-accent-text`).
3. **Selector Normalization**:
   - Replaces `.ple-<16hex>` content-addressed hashes with role-prefixed selectors (`.ui-section-*`, `.ui-grid-*`, `.ui-cell-*`, `.ui-type-*`, `.ui-btn-*`, `.ui-canvas-*`).
