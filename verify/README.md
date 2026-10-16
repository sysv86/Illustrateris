# `verify/` — headless-browser geometry harness

`ple-engine` emits HTML + CSS but its output had never been rendered by a real
browser. This harness renders a generated page in headless Chromium and reports
what the browser **actually laid out**, so engine estimates (typography scale,
line counts, section heights, grid fit) can be compared against reality instead
of against a model of reality.

Everything here is self-contained: stdlib Python + the `playwright` package,
plus a local selenium-free Chromium build. No network access is required.

```
verify/
├── browser_verify.py        # the harness (playwright sync API, headless chromium)
├── run.sh                   # thin runner + pass/fail gate (valid sh *and* python)
├── README.md
└── samples/
    ├── sample.html          # clean fixture: 12-col grid, wrapping h1, clamp() type
    └── negative_overflow.html  # deliberately broken fixture (proves the checks fire)
```

## Requirements

* `python3` with `playwright` importable:
  `python3 -c "from playwright.sync_api import sync_playwright; print('ok')"`
* Chromium browser binaries already installed in `~/.cache/ms-playwright`
  (`python3 -m playwright install chromium` once, if missing).

Nothing is built here — the HTML you want measured must already exist.

## Usage

```bash
# canonical entry point (also works as `bash verify/run.sh ...` and `sh verify/run.sh ...`)
python3 verify/run.sh verify/samples/sample.html "1440,768,375"

# measure the engine's own output, with the project fonts registered
python3 verify/run.sh /tmp/generated.html "1440,1024,768,375" \
    --fonts-dir /mnt/sdcard/Illustrateris/fonts --out-json /tmp/geometry.json

# talk to the harness directly (adds --tolerance, --settle-ms, --timeout-ms)
python3 verify/browser_verify.py --html out.html --viewport 1440,1440x2000 \
    --fonts-dir fonts --json /tmp/geometry.json
```

`run.sh` prints the JSON report to **stdout** and all problems to **stderr**.
It exits non-zero (1) when the page is dirty and 2 when it could not run at all
(missing file, harness failure). It gates on exactly four things:

1. `ok == false` (the harness itself failed),
2. any entry in `console_errors` (console errors **and** uncaught page errors),
3. any viewport with `horizontal_overflow == true`,
4. any section with `contained == false`.

`run.sh` is a polyglot: valid POSIX `sh` **and** valid Python (line 3 re-execs
`python3`), so `python3 verify/run.sh ...`, `bash verify/run.sh ...` and
`sh verify/run.sh ...` behave identically. Because it is a polyglot,
`bash -n verify/run.sh` reports a syntax error — that is lint noise only;
execution is unaffected (bash reaches `exec python3` before parsing the Python
body).

Invoking it as `./verify/run.sh` does **not** work on this machine: the workspace
lives on Android `/storage/emulated/0` through a FUSE bind mount where the
executable bit cannot be set (`chmod +x` is a silent no-op and the kernel returns
`126`), and the shebang is therefore unusable. Always name the interpreter.

## Output schema

Top level:

| field | meaning |
| --- | --- |
| `ok` | the harness completed and measured **every** requested viewport. This is tool integrity, not page quality — a clean, correct page and a horribly overflowing page both report `ok: true`. Gate on the fields below. |
| `console_errors` | every `console.error`/`console.warn`-as-error and uncaught exception, prefixed with the viewport width, e.g. `w1440: console.error: ...` |
| `warnings` | non-fatal harness notes (navigation timeout, skipped `.woff2`, malformed font file, missing HTML) |
| `fonts_injected` | families registered from `--fonts-dir`, in file order |
| `tolerance_px` | containment tolerance in use (default `1.0`) |
| `viewports[]` | one entry per requested width, in requested order |

Per viewport:

| field | meaning |
| --- | --- |
| `width` / `client_width` / `scroll_width` | viewport width as the browser saw it, `documentElement.clientWidth`, `documentElement.scrollWidth` |
| `height` | content height = `max(documentElement.scrollHeight, body.scrollHeight)` — the real page length, not the viewport height |
| `horizontal_overflow` | `scrollWidth > clientWidth + 1` |
| `overflow_px` | `max(0, scrollWidth - clientWidth)` |
| `sections[]` | `{id, rect:{x,y,w,h}, contained, overflow_px}` — `id` falls back to `section-<index>` when the element has no `id`; `contained` is true only if **every** descendant box stays inside the section's box (`position:fixed` and zero-size boxes are skipped, 1px tolerance); `overflow_px` is how far the worst offender escapes (`0` when contained) |
| `texts[]` | one entry per element that directly owns a non-whitespace text node, in DOM order |
| `fonts_used[]` | sorted, de-duplicated first family of each measured text node's declared stack (see limitations) |
| `fonts_loaded[]` | families actually in `document.fonts` with status `loaded` (only present when `--fonts-dir` was used) |
| `overflowing_elements[]` | up to 50 elements where `scrollWidth > clientWidth + 1`, worst first (ties broken by tag/text so ordering is deterministic) |

Per text node:

| field | meaning |
| --- | --- |
| `tag`, `id`, `text` | lowercased tag name, `id` or `null`, whitespace-collapsed text truncated to 160 chars |
| `has_element_children` | `true` when the element owns inline element children in addition to its text node (e.g. an action link wrapping an icon `<svg>`). `single_line_width_px` then measures text *and* icon, so such runs are not valid for a text-extent comparison and callers should skip them. |
| `section` | id of the nearest ancestor `<section>`, or `null` |
| `font_size_px`, `font_weight`, `font_family` | computed values; `font_family` is the whole declared stack |
| `color` | computed `color` |
| `background` | nearest ancestor with a non-transparent `backgroundColor` (walks up to `<html>`; `rgba(0, 0, 0, 0)` if there is none) |
| `single_line_width_px` | width of the same text laid out on **one** line: the element is cloned into an off-screen `white-space:nowrap; width:max-content` host with its font metrics copied, and the clone's rect is measured. Fallback (hidden/unclonable elements): widest single line from `Range.getClientRects()` |
| `rendered_width_px` | the element's actual border-box width |
| `line_count` | `round(contentBoxHeight / lineHeight)`; when `line-height` is `normal` (or the content box is empty) it falls back to the number of distinct text-line tops in `Range.getClientRects()` |
| `line_height_px` | resolved `line-height` in px, or `null` when the computed value is `normal` |
| `x`, `y` | document coordinates of the border box (viewport rect + scroll offset) |

`line_count` was cross-validated against the Range-rect fallback on the sample at
1440/768/375 — box-based and Range-based counts agreed on every heading
(5/5, 4/4, 3/3, 2/2).

## What a caller should assert on

For every `viewport`:

```python
assert vp["horizontal_overflow"] is False
assert vp["overflow_px"] == 0
assert all(s["contained"] for s in vp["sections"])
assert all(err is None for err in data["console_errors"] or [None])   # i.e. data["console_errors"] == []
assert vp["height"] > 0

# typography versus the engine's estimate (join on id, or tag+text)
for t in vp["texts"]:
    if t["id"] == "hero-title":
        assert abs(t["font_size_px"] - expected_px) < 0.5      # clamp() resolution
        assert t["line_count"] == engine_estimate.line_count
        assert t["single_line_width_px"] <= t["rendered_width_px"] * 1.02   # guesses that fit
```

Useful comparisons this harness specifically enables:

* **clamp() resolution** — `fonts_used`/`font_size_px` at 1440 vs 768 vs 375 shows
  which side of the clamp a heading landed on.
* **line-count drift** — `single_line_width_px / rendered_width_px` predicts how
  many lines a string needs; compare with `line_count` to catch "the engine
  thought this was one line".
* **contrast pairs** — `color` + `background` are the *effective* rendered pair
  (background walked up the tree), exactly what a contrast check should use.
* **grid fit** — `sections[].contained == false` or `overflowing_elements` means
  the chosen tracks/gaps do not hold the content at that width.

## Proving the checks actually fire

`samples/negative_overflow.html` contains a `width:2200px` child and an
absolutely positioned child placed at `left: calc(100% + 60px)`:

```console
$ python3 verify/run.sh verify/samples/negative_overflow.html "1440,768"
verify: FAIL: width 1440: horizontal overflow of 784px
verify: FAIL: width 1440: section #wide not contained (child escapes by 784px)
verify: FAIL: width 1440: section #escape not contained (child escapes by 260px)
verify: FAIL: width 768: horizontal overflow of 1456px
...
verify: FAIL: 6 problem(s) across 2 viewport(s)
$ echo $?
1
```

The harness reports the same facts in JSON (`"horizontal_overflow": true`,
`"overflow_px": 784`, `"contained": false`), so this fixture is a permanent
regression proof that the gate is not a rubber stamp.

## Determinism

Output ordering is deterministic (viewports in requested order, sections/texts in
DOM order, families and overflow lists sorted). Repeated runs on the same input
and Chromium build produce byte-identical JSON — verified with SHA-256 over two
consecutive runs of `--viewport 1440,375`.

Measurement runs at `device_scale_factor=1` with `--hide-scrollbars` and a fresh
page per viewport, so results do not depend on window size, DPI or prior state.

## Fonts

Skip-based fallback silently destroys typography measurements: if the page asks
for `Inter` and the browser only has Arial, every width differs. `--fonts-dir`
walks the directory (recursively), reads each font's `name`/`OS/2`/`head` tables
directly (`ttf`, `otf`, `ttc`, `woff` — `woff2` needs brotli, not in the stdlib,
so it is skipped with a warning), injects matching `@font-face` rules, waits for
`document.fonts.ready`, then measures.

Measured effect on `samples/sample.html` at 1440px:

| | `h1` single-line width | `h1` line count |
| --- | --- | --- |
| no `--fonts-dir` (Inter → Arial fallback) | 3247.2px | 5 |
| `--fonts-dir fonts` (real Inter loaded) | 2745.4px | 4 |

`fonts_loaded` lists only faces the page actually requested; unused faces from the
directory stay `unloaded`.

## Limitations / things to know

* `fonts_used` is the **declared** first family of each stack — browsers expose no
  API for the actually-used face, so verify with `fonts_loaded` (populated only
  for `--fonts-dir` runs) or by diffing measurements with and without the font.
  The UA stylesheet does not inherit `font-family` into `<button>`, so buttons
  legitimately report `Arial` while the body reports `Inter` — worth fixing in the
  engine, not in this harness.
* `.woff2` from `--fonts-dir` is skipped (no brotli in the stdlib). Convert to
  `.ttf`/`.otf`/`.woff`, or reference the file directly from the page.
* `line_height_px` is `null` when `line-height` is `normal`; `line_count` then
  comes from `Range` rects.
* Text nodes that belong to replaced elements (`<input>`, `<textarea>`) are not
  reportable — they own no text node; only `<button>`-style labels appear.
* `contained` compares each descendant against the section box only, not against
  the grid track it sits in.
* The workspace is on a FUSE bind mount without exec permission, so the harness
  must be started through an interpreter (`python3 …` / `bash …`); see above.
* Not yet exercised against engine-produced HTML: no `ple` binary was available
  in `target/` and building the crate was out of this task's scope. The harness
  itself is validated against the clean sample, the broken fixture, and a
  real-font variant (`--fonts-dir fonts`).
* The engine's default HTML embeds the Tailwind browser build from a CDN
  (`HtmlOptions { tailwind_cdn: true }`, `src/codegen/html.rs:15`). Measuring that
  file offline yields `console_errors` for the failed fetch **and** an unstyled
  page, so the geometry is meaningless. Measure online, or generate the HTML
  without the CDN script and with styles resolved locally.
