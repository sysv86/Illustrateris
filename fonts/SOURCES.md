# Bundled reference fonts

These are the **static** Regular (400) and Bold (700) faces used by
`tools/gen_font_metrics.py` to extract per-glyph advance widths into
`src/text/metrics_data.rs`. They are bundled for reproducible metric
generation; the engine does not load them at runtime.

All three families are licensed under the **SIL Open Font License 1.1**
(permissive, redistribution and bundling allowed). The complete license text
shipped with each release is kept next to the binaries.

No substitution was necessary: every requested family — Inter, Source Serif 4
and JetBrains Mono — publishes downloadable static Regular/Bold `.ttf` faces.

Downloaded: 2026-10-15 (UTC).

---

## sans — Inter 4.1 (static)

- **Family name (name table):** `Inter`
- **Embedded version string:** `Version 4.001;git-9221beed3` (release archive `v4.1`)
- **Designer:** Rasmus Andersson
- **License:** SIL Open Font License 1.1 — see [`sans/Inter-LICENSE.txt`](sans/Inter-LICENSE.txt)
- **Upstream:** <https://github.com/rsms/inter>
- **Release page:** <https://github.com/rsms/inter/releases/tag/v4.1>
- **Archive:** <https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip>
  - archive sha256 `9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e`

| Bundled file | Path inside archive | sha256 |
| --- | --- | --- |
| `sans/Inter-Regular.ttf` | `extras/ttf/Inter-Regular.ttf` | `40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82` |
| `sans/Inter-Bold.ttf` | `extras/ttf/Inter-Bold.ttf` | `288316099b1e0a47a4716d159098005eef7c0066921f34e3200393dbdb01947f` |

> Note: the release is tagged `v4.1` but the static instances carry the
> internal version `4.001`. `metrics_data.rs` records the same hashes.

---

## serif — Source Serif 4.005R (static)

- **Family name (name table):** `Source Serif 4`
- **Embedded version string:** `Version 4.005;hotconv 1.1.0;makeotfexe 2.6.0`
- **Designer:** Frank Grießhammer
- **License:** SIL Open Font License 1.1 — see [`serif/SourceSerif4-LICENSE.md`](serif/SourceSerif4-LICENSE.md)
- **Upstream:** <https://github.com/adobe-fonts/source-serif>
- **Release page:** <https://github.com/adobe-fonts/source-serif/releases/tag/4.005R>
- **Archive:** <https://github.com/adobe-fonts/source-serif/releases/download/4.005R/source-serif-4.005_Desktop.zip>
  - archive sha256 `549fdb8f9a682bd06944298621404969f6de77c2e422ff3b8244a1dcd6a0c425`

| Bundled file | Path inside archive | sha256 |
| --- | --- | --- |
| `serif/SourceSerif4-Regular.ttf` | `source-serif-4.005_Desktop/TTF/SourceSerif4-Regular.ttf` | `e5a4ee6a3d87bb9024796be390c6771e2a0eb1883dae25effaf57ca01668e24b` |
| `serif/SourceSerif4-Bold.ttf` | `source-serif-4.005_Desktop/TTF/SourceSerif4-Bold.ttf` | `7cf4f4e1ad74f45058d5bc61716b82560442fbdcd9d3654d2dea96bf6c683d86` |

> The archive ships `TTF/` (glyf) and `OTF/` (CFF) builds of the same family;
> the **TTF** build was chosen for consistency with the other two families.

---

## mono — JetBrains Mono 2.304 (static)

- **Family name (name table):** `JetBrains Mono`
- **Embedded version string:** `Version 2.304; ttfautohint (v1.8.4.7-5d5b)`
- **Designers:** Philipp Nurullin, Konstantin Bulenkov
- **License:** SIL Open Font License 1.1 — see [`mono/JetBrainsMono-OFL.txt`](mono/JetBrainsMono-OFL.txt)
- **Upstream:** <https://github.com/JetBrains/JetBrainsMono>
- **Release page:** <https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304>
- **Archive:** <https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip>
  - archive sha256 `6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf`

| Bundled file | Path inside archive | sha256 |
| --- | --- | --- |
| `mono/JetBrainsMono-Regular.ttf` | `fonts/ttf/JetBrainsMono-Regular.ttf` | `a0bf60ef0f83c5ed4d7a75d45838548b1f6873372dfac88f71804491898d138f` |
| `mono/JetBrainsMono-Bold.ttf` | `fonts/ttf/JetBrainsMono-Bold.ttf` | `5590990c82e097397517f275f430af4546e1c45cff408bde4255dad142479dcb` |

> The `*NL` ("no ligatures") variants were **not** used; the regular
> `JetBrainsMono-*` faces were chosen. Ligature substitution is irrelevant to
> advance-width extraction, which reads `hmtx` directly.

---

## Regenerating

```sh
python3 tools/gen_font_metrics.py --fonts fonts --out src/text/metrics_data.rs
```

The generator is pure Python 3 (stdlib only) and parses `head`, `maxp`, `hhea`,
`hmtx` and `cmap` (formats 4 and 12) directly. Output is deterministic:
re-running on these exact files reproduces `metrics_data.rs` byte for byte
(verified sha256 `a2fe1178d8859aebe274f3595b5948af3bf3084cd9f98167b7d2c7ea14e36434`).

### Units per em (as read from `head.unitsPerEm`)

| Family | upm |
| --- | --- |
| Inter | 2048 |
| Source Serif 4 | 1000 |
| JetBrains Mono | 1000 |
