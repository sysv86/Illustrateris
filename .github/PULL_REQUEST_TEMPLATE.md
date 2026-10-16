## Summary

<!-- What changes, and why. One paragraph. -->

## Related issue

<!-- fixes #123 -->

## Type of change

- [ ] Bug fix (non-breaking)
- [ ] New feature (non-breaking)
- [ ] Breaking change to the CLI or JSON contract
- [ ] Documentation / examples only
- [ ] CI / packaging only

## How it was tested

<!-- Exact commands, and their result. Do not write "tests pass" without the command. -->

```sh
cargo build --locked --all-targets
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Checklist

- [ ] `cargo test --locked` passes locally on stable.
- [ ] `cargo clippy --locked --all-targets -- -D warnings` is clean.
- [ ] Output is still deterministic (repeat runs are byte-identical; no timestamps added).
- [ ] Contrast targets still hold (`palette.all_checks_passed` is `true` for every example).
- [ ] Every changed layout-affecting field is covered by a test or an example under `examples/`.
- [ ] If `tools/gen_font_metrics.py` changed, `src/text/metrics_data.rs` was regenerated with it.
- [ ] `CHANGELOG.md` has an entry under the unreleased/next version.
- [ ] I did not reformat unrelated code (this repository keeps a hand-maintained compact style).
