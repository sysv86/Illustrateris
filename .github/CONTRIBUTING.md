# Contributing

Thanks for taking the time to improve Illustrateris. This document describes how the repository is
built, what the CI checks, and the conventions a change is expected to follow.

## Layout

| Path | What it is |
| --- | --- |
| `src/` | The `ple-engine` crate: context parsing, profiler, OKLCH/APCA colour, tokens, BSP layout, solver, IR, codegen. |
| `examples/` | Context JSON documents used by the test suite and the CLI. |
| `tests/` | Integration tests (`pipeline.rs`) and the opt-in headless-browser test (`browser.rs`). |
| `verify/` | The headless-Chromium geometry harness and its fixtures. |
| `tools/` | `gen_font_metrics.py`, which regenerates `src/text/metrics_data.rs` from the bundled fonts. |
| `fonts/` | The reference faces (OFL-1.1) and their provenance in `SOURCES.md`. |
| `skills/` | Agent Skills that wrap the engine. |

## Requirements

- Rust. The declared MSRV is **1.70** (`rust-version` in `Cargo.toml`); CI tests both stable and 1.70.
- Python 3 for `tools/` and `verify/`. The metrics generator is stdlib-only.
- Playwright + Chromium only if you want to run the browser verification locally.

The crate has **no runtime dependencies by design**. A pull request that adds one needs a strong
justification and will be reviewed against the "zero dependencies, build without network" property.

## Build and test

```sh
cargo build --locked --all-targets
cargo test  --locked
cargo clippy --locked --all-targets -- -D warnings
```

The browser test is opt-in because the default `cargo test` must stay hermetic:

```sh
python3 -m pip install playwright
python3 -m playwright install chromium
PLE_BROWSER=1 cargo test --locked --test browser
```

Regenerating the metric tables (must reproduce `src/text/metrics_data.rs` byte for byte):

```sh
python3 tools/gen_font_metrics.py --fonts fonts --out src/text/metrics_data.rs
```

## What CI runs

`.github/workflows/ci.yml` runs on every push to `main`, on pull requests and on demand:

- **build + test** on stable (`cargo build/test --locked`);
- **clippy** with `-D warnings`;
- **MSRV** build and test on 1.70.0;
- **tooling** — regenerates `metrics_data.rs` and fails if it differs from the committed file, then
  byte-compiles the Python tooling;
- **browser** — installs Playwright/Chromium and runs `PLE_BROWSER=1 cargo test --test browser`.

`rustfmt` is **not** enforced. The source uses a deliberately compact, hand-maintained style (and
`src/text/metrics_data.rs` is generated), so `cargo fmt` would rewrite the whole tree and break the
generator's byte-for-byte guarantee. Match the surrounding code instead of reformatting unrelated
lines.

## Invariants a change must preserve

1. **Determinism.** Output is a pure function of `(context JSON, seed, config)`. No timestamps, no
   hash-iteration order. Two runs must be byte-identical.
2. **Contrast safety.** Every emitted colour meets its APCA `|Lc|` target on every background it can
   appear on. `report.palette.all_checks_passed` must stay `true` for all examples.
3. **Grid safety.** Everything sits on the integer 12-column grid; children tile their parent with no
   overlaps and no gaps inside a split.
4. **Content awareness.** Column widths come from real per-glyph advances, not fixed spans.
5. **Escaping.** All user text passes through one escaping function; tags come from closed enums.

If a change cannot preserve one of these, say so explicitly in the pull request.

## Commits and pull requests

- Keep commits focused: one bug fix, one feature, one docs change.
- Use a conventional subject, e.g. `fix(solver): reject degenerate splits` or
  `feat(codegen): emit <picture> for photo anchors`.
- Describe the affected files, the behaviour change and the test that covers it. No marketing copy.
- Fill in the pull request template, including the exact commands you ran.
- Update `CHANGELOG.md` for user-visible changes.
- Do not commit build output (`target/`), fonts you did not add provenance for, or secrets.
