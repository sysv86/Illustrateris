#!/usr/bin/env python3
"""eject_ple.py -- Strip ple-engine footprints and eject generated HTML/CSS into clean tokens.

Removes all traces of `ple-engine` from generated HTML/CSS/JSX files:
  1. Strips `<!-- ple-engine ... -->` comments and `<meta name="generator" content="ple-engine...">`.
  2. Extracts repeated inline `oklch(...)` colours and font stacks into `:root` CSS Custom Properties
     (`--color-bg`, `--color-surface`, `--color-text`, `--color-border`, `--color-text-muted`,
      `--color-accent`, `--color-accent-text`) so a human or AI agent can restyle the page without
     re-running `ple`.
  3. Renames content-addressed `.ple-<16hex>` classes into clean, semantic classes
     (`.ui-section-1`, `.ui-grid-2`, `.ui-cell-3`, `.ui-type-4`, `.ui-btn-5`, `.ui-canvas-6`).

Usage:
    python3 skills/ple-engine/scripts/eject_ple.py input.html -o clean.html
    python3 skills/ple-engine/scripts/eject_ple.py input.html --in-place
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import OrderedDict


_PLE_COMMENT_RE = re.compile(r"<!--\s*ple-engine[^>]*-->\s*\n?", re.IGNORECASE)
_PLE_META_RE = re.compile(
    r'\s*<meta\s+name=["\']generator["\']\s+content=["\']ple-engine[^"\']*["\']\s*/?>\s*\n?',
    re.IGNORECASE,
)
_PLE_CLASS_RE = re.compile(r"\bple-([0-9a-f]{16})\b")
_OKLCH_RE = re.compile(r"oklch\([^)]+\)")


def _infer_semantic_prefix(rule_body: str, index: int) -> str:
    if "display:grid" in rule_body and "repeat(12" in rule_body:
        return f"ui-grid-{index}"
    if "position:relative;width:100%;overflow:hidden;display:flex;flex-direction:column" in rule_body:
        return f"ui-section-{index}"
    if "grid-column:1 / -1" in rule_body:
        return f"ui-cell-{index}"
    if "aspect-ratio:" in rule_body:
        return f"ui-canvas-{index}"
    if "display:inline-flex" in rule_body and "background-color:" in rule_body:
        return f"ui-btn-{index}"
    if "font-size:" in rule_body and "line-height:" in rule_body:
        return f"ui-type-{index}"
    if "display:flex" in rule_body:
        return f"ui-stack-{index}"
    return f"ui-el-{index}"


def eject_html(html: str) -> tuple[str, dict[str, str]]:
    """Return (clean_html, stats) with all ple-engine footprints removed."""
    out = _PLE_COMMENT_RE.sub("", html)
    out = _PLE_META_RE.sub("\n", out)

    class_map: OrderedDict[str, str] = OrderedDict()
    rule_bodies: dict[str, str] = {}

    for match in re.finditer(r"\.(ple-[0-9a-f]{16})\s*\{([^}]*)\}", out):
        old_cls, body = match.group(1), match.group(2)
        if old_cls not in rule_bodies:
            rule_bodies[old_cls] = body

    for m in _PLE_CLASS_RE.finditer(out):
        full = f"ple-{m.group(1)}"
        if full not in class_map:
            body = rule_bodies.get(full, "")
            class_map[full] = _infer_semantic_prefix(body, len(class_map) + 1)

    for old_cls, new_cls in class_map.items():
        out = re.sub(r"\b" + re.escape(old_cls) + r"\b", new_cls, out)

    oklch_seen: OrderedDict[str, str] = OrderedDict()
    role_names = [
        "--color-bg",
        "--color-surface",
        "--color-text",
        "--color-border",
        "--color-text-muted",
        "--color-accent",
        "--color-accent-text",
    ]
    for m in _OKLCH_RE.finditer(out):
        val = m.group(0)
        if val not in oklch_seen:
            idx = len(oklch_seen)
            var_name = role_names[idx] if idx < len(role_names) else f"--color-tok-{idx + 1}"
            oklch_seen[val] = var_name

    if oklch_seen and ":root" not in out and "<style>" in out:
        root_lines = [":root {"]
        for val, var_name in oklch_seen.items():
            root_lines.append(f"    {var_name}: {val};")
        root_lines.append("  }")
        root_block = "\n  ".join(root_lines) + "\n"
        for val, var_name in oklch_seen.items():
            out = out.replace(f":{val}", f":var({var_name})")
            out = out.replace(f"solid {val}", f"solid var({var_name})")
        out = out.replace("<style>\n", "<style>\n  " + root_block, 1)

    stats = {
        "classes_renamed": str(len(class_map)),
        "tokens_extracted": str(len(oklch_seen)),
    }
    return out, stats


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Strip ple-engine footprints and eject to clean CSS tokens.")
    ap.add_argument("input", help="Input HTML/CSS file ('-' for stdin)")
    ap.add_argument("-o", "--out", help="Output file path (default: stdout)")
    ap.add_argument("--in-place", action="store_true", help="Modify the input file in place")
    args = ap.parse_args(argv)

    if args.input == "-":
        raw = sys.stdin.read()
    else:
        with open(args.input, "r", encoding="utf-8") as fh:
            raw = fh.read()

    cleaned, stats = eject_html(raw)

    if args.in_place and args.input != "-":
        with open(args.input, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(cleaned)
    elif args.out:
        with open(args.out, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(cleaned)
    else:
        sys.stdout.write(cleaned)

    sys.stderr.write(
        f"eject_ple: ok ({stats['classes_renamed']} ple-* classes renamed, "
        f"{stats['tokens_extracted']} OKLCH tokens extracted)\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
