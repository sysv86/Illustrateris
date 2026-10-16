#!/usr/bin/env python3
"""browser_verify.py -- measure the *real* rendered geometry of a local HTML file.

Loads an HTML file in headless Chromium (Playwright sync API) at one or more
viewport widths and reports what the browser actually laid out:

  * per-viewport document content height
  * horizontal overflow (documentElement.scrollWidth vs clientWidth)
  * per-<section> containment (every descendant box inside the section box)
  * per-text-node typography, effective background, single-line width,
    rendered width, line count and line height
  * declared font families and elements whose content overflows their own box

Usage
-----
  python3 verify/browser_verify.py --html out.html \
      [--viewport 1440,768,375] [--fonts-dir fonts/] [--json geometry.json]

Notes
-----
* stdlib + `playwright` only; no network access is required or attempted.
* Output ordering is deterministic: viewports in the order requested, sections
  and texts in DOM order, families sorted.
* Exit status: 0 when the harness completed (`ok: true`), 2 when it could not
  (missing file, navigation/measurement failure).  A *dirty page* (overflow,
  console errors) is reported in the JSON; use `verify/run.sh` to gate on it.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import struct
import sys
import zlib

TOOL = "ple-verify/browser_verify.py"

DEFAULT_VIEWPORTS = "1440,768,375"
DEFAULT_VIEWPORT_HEIGHT = 900


# --------------------------------------------------------------------------- #
# fonts-dir support: read the `name` table out of font files so we can register
# the faces under their real family name before measuring.
# --------------------------------------------------------------------------- #

_SFNT_HEADER = 12
_SFNT_RECORD = 16


def _sfnt_tables(data: bytes, base: int) -> dict[str, bytes]:
    if len(data) < base + _SFNT_HEADER:
        raise ValueError("truncated sfnt header")
    num_tables = struct.unpack_from(">H", data, base + 4)[0]
    tables: dict[str, bytes] = {}
    off = base + _SFNT_HEADER
    for _ in range(num_tables):
        if off + _SFNT_RECORD > len(data):
            break
        tag = data[off : off + 4].decode("latin-1")
        toff, tlen = struct.unpack_from(">II", data, off + 8)
        if toff + tlen <= len(data):
            tables[tag] = data[toff : toff + tlen]
        off += _SFNT_RECORD
    return tables


def _woff_tables(data: bytes) -> dict[str, bytes]:
    if len(data) < 44:
        raise ValueError("truncated woff header")
    num_tables = struct.unpack_from(">H", data, 12)[0]
    tables: dict[str, bytes] = {}
    off = 44
    for _ in range(num_tables):
        if off + 20 > len(data):
            break
        tag = data[off : off + 4].decode("latin-1")
        toff, comp_len, orig_len = struct.unpack_from(">III", data, off + 4)
        raw = data[toff : toff + comp_len]
        if len(raw) != orig_len:
            try:
                raw = zlib.decompress(raw)
            except zlib.error:
                raw = b""
        if raw:
            tables[tag] = raw
        off += 20
    return tables


def _font_tables(path: str) -> dict[str, bytes]:
    """Return {table_tag: table_bytes} for .ttf/.otf/.ttc/.woff (no .woff2)."""
    with open(path, "rb") as fh:
        data = fh.read()
    if data[:4] == b"ttcf":
        base = struct.unpack_from(">I", data, 12)[0]
        return _sfnt_tables(data, base)
    if data[:4] == b"wOFF":
        return _woff_tables(data)
    return _sfnt_tables(data, 0)


def _decode_name(raw: bytes, platform_id: int, encoding_id: int) -> str:
    if platform_id == 3 or (platform_id == 0):  # Windows / Unicode -> UTF-16BE
        return raw.decode("utf-16-be", "replace")
    return raw.decode("latin-1", "replace")


def _font_metadata(path: str) -> dict | None:
    """(family, subfamily, weight, italic) for a font file, or None."""
    try:
        tables = _font_tables(path)
    except Exception:
        return None
    names: dict[int, str] = {}
    blob = tables.get("name")
    if blob and len(blob) >= 6:
        try:
            _fmt, count, str_off = struct.unpack_from(">HHH", blob, 0)
            for i in range(count):
                p = 6 + i * 12
                if p + 12 > len(blob):
                    break
                pid, eid, _lid, nid, length, noff = struct.unpack_from(">HHHHHH", blob, p)
                if nid not in (1, 2, 16, 17):
                    continue
                raw = blob[str_off + noff : str_off + noff + length]
                if not raw:
                    continue
                val = _decode_name(raw, pid, eid)
                if val and nid not in names:
                    names[nid] = val
        except Exception:
            pass
    family = names.get(16) or names.get(1)
    if not family:
        return None
    subfamily = names.get(17) or names.get(2) or ""
    weight = 400
    os2 = tables.get("OS/2")
    if os2 and len(os2) >= 6:
        try:
            w = struct.unpack_from(">H", os2, 4)[0]
            if 1 <= w <= 1000:
                weight = int(w)
        except Exception:
            pass
    italic = "italic" in subfamily.lower() or "oblique" in subfamily.lower()
    head = tables.get("head")
    if head and len(head) >= 46:
        try:
            if struct.unpack_from(">H", head, 44)[0] & 0x02:
                italic = True
        except Exception:
            pass
    return {"family": family, "subfamily": subfamily, "weight": weight, "italic": italic}


def _font_format(path: str) -> str:
    ext = os.path.splitext(path)[1].lower()
    return {
        ".otf": "opentype",
        ".ttf": "truetype",
        ".ttc": "truetype",
        ".woff": "woff",
    }.get(ext, "truetype")


def collect_font_faces(dirs: list[str]) -> tuple[str, list[str], list[str]]:
    """Build @font-face CSS for every font file found under `dirs`.

    Returns (css, injected_families, warnings).
    """
    exts = {".ttf", ".otf", ".ttc", ".woff", ".woff2"}
    files: list[str] = []
    for d in dirs:
        for root, _sub, names in os.walk(d):
            for n in sorted(names):
                if os.path.splitext(n)[1].lower() in exts:
                    files.append(os.path.join(root, n))
    rules: list[str] = []
    families: list[str] = []
    warnings: list[str] = []
    for path in sorted(files):
        ext = os.path.splitext(path)[1].lower()
        if ext == ".woff2":
            warnings.append(
                f"fonts-dir: skipped {path} (.woff2 needs brotli, not in stdlib); "
                "convert to .ttf/.otf/.woff"
            )
            continue
        meta = _font_metadata(path)
        if not meta:
            warnings.append(f"fonts-dir: no usable name table in {path}; skipped")
            continue
        fam = meta["family"].replace("\\", "").replace("'", "")
        style = "italic" if meta["italic"] else "normal"
        url = "file://" + os.path.abspath(path)
        rules.append(
            "@font-face{font-family:'%s';font-style:%s;font-weight:%d;"
            "font-display:block;src:url('%s') format('%s')}"
            % (fam, style, meta["weight"], url, _font_format(path))
        )
        families.append(fam)
    css = "\n".join(rules)
    # deterministic de-dup, order preserved
    seen = set()
    uniq = [f for f in families if not (f in seen or seen.add(f))]
    return css, uniq, warnings


# --------------------------------------------------------------------------- #
# in-page measurement
# --------------------------------------------------------------------------- #

MEASURE_JS = r"""
(tol) => {
  const round1 = (v) => Math.round(v * 10) / 10;
  const clip = (s, n) => {
    s = (s || '').replace(/\s+/g, ' ').trim();
    return s.length > n ? s.slice(0, n - 1) + '…' : s;
  };
  const SKIP = new Set(['script', 'style', 'template', 'head', 'title', 'meta', 'link', 'html', 'br']);
  const scrollX = window.scrollX || 0;
  const scrollY = window.scrollY || 0;
  const docEl = document.documentElement;

  const tagOf = (el) => el.tagName.toLowerCase();
  const firstFamily = (cs) => {
    const raw = cs.fontFamily || '';
    const parts = raw.split(',').map((s) => s.trim().replace(/^["']|["']$/g, ''));
    for (const p of parts) if (p) return p;
    return '';
  };
  const effectiveBackground = (el) => {
    let n = el;
    while (n && n.nodeType === 1) {
      const bg = getComputedStyle(n).backgroundColor;
      if (bg && bg !== 'transparent' && bg !== 'rgba(0, 0, 0, 0)') return bg;
      n = n.parentElement;
    }
    return 'rgba(0, 0, 0, 0)';
  };
  const owningSection = (el) => {
    const s = el.closest('section');
    if (!s) return null;
    return s.id || ('section-' + Array.prototype.indexOf.call(document.querySelectorAll('section'), s));
  };

  // ---- single-line width: clone into an off-screen nowrap host -------------
  const CLONE_PROPS = ['fontFamily', 'fontSize', 'fontWeight', 'fontStyle', 'fontStretch',
    'fontVariant', 'fontFeatureSettings', 'fontKerning', 'fontVariationSettings',
    'letterSpacing', 'wordSpacing', 'textTransform', 'textIndent', 'textRendering',
    'direction', 'unicodeBidi', 'tabSize', 'overflowWrap', 'wordBreak'];
  const host = document.createElement('div');
  host.setAttribute('data-ple-measure-host', '1');
  host.style.cssText = 'position:absolute;left:-100000px;top:0;width:max-content;' +
    'max-width:none;height:auto;visibility:hidden;pointer-events:none;z-index:-1;contain:layout style;';
  document.body.appendChild(host);

  const singleLineWidth = (el) => {
    const cs = getComputedStyle(el);
    const clone = el.cloneNode(true);
    clone.removeAttribute('id');
    clone.style.cssText = 'display:inline-block;white-space:nowrap;width:max-content;' +
      'max-width:none;min-width:0;margin:0;padding:0;border:0;';
    for (const p of CLONE_PROPS) { try { clone.style[p] = cs[p]; } catch (e) {} }
    clone.style.whiteSpace = 'nowrap';
    clone.style.display = 'inline-block';
    clone.style.width = 'max-content';
    host.replaceChildren(clone);
    let w = 0;
    try { w = clone.getBoundingClientRect().width; } catch (e) { w = 0; }
    host.replaceChildren();
    return w;
  };
  // fallback for elements a clone cannot reproduce: widest single line
  const rangeMaxLineWidth = (el) => {
    try {
      const r = document.createRange();
      r.selectNodeContents(el);
      let max = 0;
      for (const rect of r.getClientRects()) if (rect.width > max) max = rect.width;
      return max;
    } catch (e) { return 0; }
  };
  const rangeLineCount = (el) => {
    try {
      const r = document.createRange();
      r.selectNodeContents(el);
      const tops = [];
      for (const rect of r.getClientRects()) {
        if (rect.height > 0.5 && rect.width > 0.5) tops.push(Math.round(rect.top));
      }
      if (!tops.length) return 0;
      tops.sort((a, b) => a - b);
      let lines = 1;
      for (let i = 1; i < tops.length; i++) if (Math.abs(tops[i] - tops[i - 1]) > 2) lines++;
      return lines;
    } catch (e) { return 0; }
  };

  // ---- texts ---------------------------------------------------------------
  const texts = [];
  for (const el of document.querySelectorAll('*')) {
    if (texts.length >= 500) break;
    const tag = tagOf(el);
    if (SKIP.has(tag)) continue;
    let hasOwnText = false;
    for (const n of el.childNodes) {
      if (n.nodeType === 3 && n.nodeValue && n.nodeValue.trim().length) { hasOwnText = true; break; }
    }
    if (!hasOwnText) continue;
    const rect = el.getBoundingClientRect();
    if (rect.width <= 0 && rect.height <= 0) continue;
    const cs = getComputedStyle(el);
    const fontPx = parseFloat(cs.fontSize) || 0;
    const lhRaw = parseFloat(cs.lineHeight);
    const lineHeightPx = Number.isFinite(lhRaw) && lhRaw > 0 ? round1(lhRaw) : null;
    const padT = parseFloat(cs.paddingTop) || 0;
    const padB = parseFloat(cs.paddingBottom) || 0;
    const contentH = Math.max(0, el.clientHeight - padT - padB);
    const rLines = rangeLineCount(el);
    let lineCount = 1;
    if (contentH > 0 && lineHeightPx) lineCount = Math.max(1, Math.round(contentH / lineHeightPx));
    else if (rLines > 0) lineCount = rLines;
    let slw = singleLineWidth(el);
    if (!(slw > 0)) slw = rangeMaxLineWidth(el);
    // An element can own a text node *and* inline element children (e.g. an icon `<svg>`
    // inside an action link). `single_line_width_px` then measures text + children, so a
    // pure text-extent comparison must be able to tell the two apart.
    let hasElementChildren = false;
    for (const n of el.childNodes) { if (n.nodeType === 1) { hasElementChildren = true; break; } }
    texts.push({
      tag: tag,
      id: el.id || null,
      section: owningSection(el),
      text: clip(el.textContent, 160),
      has_element_children: hasElementChildren,
      font_size_px: round1(fontPx),
      font_family: cs.fontFamily || '',
      font_weight: cs.fontWeight || '',
      color: cs.color || '',
      background: effectiveBackground(el),
      single_line_width_px: round1(slw),
      rendered_width_px: round1(rect.width),
      line_count: lineCount,
      line_height_px: lineHeightPx,
      x: round1(rect.left + scrollX),
      y: round1(rect.top + scrollY),
    });
  }

  // ---- sections ------------------------------------------------------------
  const sections = [];
  const allSections = Array.from(document.querySelectorAll('section'));
  allSections.forEach((sec, idx) => {
    const sr = sec.getBoundingClientRect();
    let contained = true;
    let worst = 0;
    for (const d of sec.querySelectorAll('*')) {
      const t = tagOf(d);
      if (SKIP.has(t)) continue;
      const dcs = getComputedStyle(d);
      if (dcs.display === 'none' || dcs.visibility === 'hidden' || dcs.position === 'fixed') continue;
      const r = d.getBoundingClientRect();
      if (r.width <= 0 && r.height <= 0) continue;
      const over = Math.max(sr.left - r.left, r.right - sr.right, sr.top - r.top, r.bottom - sr.bottom);
      if (over > tol) { contained = false; worst = Math.max(worst, over); }
    }
    sections.push({
      id: sec.id || ('section-' + idx),
      rect: { x: round1(sr.left + scrollX), y: round1(sr.top + scrollY), w: round1(sr.width), h: round1(sr.height) },
      contained: contained,
      overflow_px: round1(worst),
    });
  });

  // ---- document-level overflow --------------------------------------------
  const clientW = docEl.clientWidth;
  const scrollW = docEl.scrollWidth;
  const overflowPx = Math.max(0, scrollW - clientW);
  const overflowing = [];
  for (const el of document.querySelectorAll('*')) {
    const t = tagOf(el);
    if (SKIP.has(t)) continue;
    const sw = el.scrollWidth;
    const cw = el.clientWidth;
    const ch = el.clientHeight;
    // A one-pixel stub is the canonical visually-hidden accessibility helper (`sr-only`). It is
    // deliberately clipped: it never renders the text it carries, so it is not clipped content.
    if (cw <= 1 || ch <= 1) continue;
    if (sw > cw + 1) {
      overflowing.push({
        tag: t, scrollWidth: sw, clientWidth: cw,
        text: clip(el.textContent, 80),
      });
    }
  }
  overflowing.sort((a, b) =>
    (b.scrollWidth - b.clientWidth) - (a.scrollWidth - a.clientWidth) ||
    a.tag.localeCompare(b.tag) || a.text.localeCompare(b.text));

  const families = [];
  for (const t of texts) {
    const f = firstFamily({ fontFamily: t.font_family });
    if (f && families.indexOf(f) === -1) families.push(f);
  }
  families.sort();

  const loaded = [];
  try {
    document.fonts.forEach((ff) => {
      if (ff.status === 'loaded' && loaded.indexOf(ff.family) === -1) loaded.push(ff.family);
    });
  } catch (e) { /* FontFaceSet unavailable */ }
  loaded.sort();

  return {
    width: window.innerWidth,
    height: Math.max(docEl.scrollHeight, document.body ? document.body.scrollHeight : 0),
    horizontal_overflow: scrollW > clientW + 1,
    overflow_px: overflowPx,
    scroll_width: scrollW,
    client_width: clientW,
    sections: sections,
    texts: texts,
    fonts_used: families,
    fonts_loaded: loaded,
    overflowing_elements: overflowing.slice(0, 50),
  };
}
"""


# --------------------------------------------------------------------------- #
# driver
# --------------------------------------------------------------------------- #


def parse_viewports(spec: str, default_h: int) -> list[tuple[int, int]]:
    out: list[tuple[int, int]] = []
    seen = set()
    for part in spec.split(","):
        part = part.strip()
        if not part:
            continue
        m = re.match(r"^(\d+)(?:[:xX](\d+))?$", part)
        if not m:
            raise ValueError(f"bad viewport spec {part!r} (want WIDTH or WIDTHxHEIGHT)")
        w = int(m.group(1))
        h = int(m.group(2)) if m.group(2) else default_h
        if (w, h) in seen:
            continue
        seen.add((w, h))
        out.append((w, h))
    if not out:
        raise ValueError("no viewports requested")
    return out


def run(args: argparse.Namespace) -> tuple[dict, int]:
    from playwright.sync_api import TimeoutError as PWTimeout, sync_playwright

    html_path = os.path.abspath(args.html)
    result: dict = {
        "tool": TOOL,
        "html": html_path,
        "ok": False,
        "console_errors": [],
        "warnings": [],
        "fonts_injected": [],
        "tolerance_px": args.tolerance,
        "viewports": [],
    }

    if not os.path.isfile(html_path):
        result["warnings"].append(f"html file not found: {html_path}")
        return result, 2

    try:
        viewports = parse_viewports(args.viewport, args.viewport_height)
    except ValueError as exc:
        result["warnings"].append(str(exc))
        return result, 2

    css = ""
    if args.fonts_dir:
        css, injected, warns = collect_font_faces(args.fonts_dir)
        result["fonts_injected"] = injected
        result["warnings"].extend(warns)
        result["fonts_dirs"] = [os.path.abspath(d) for d in args.fonts_dir]

    url = "file://" + html_path
    errors: list[str] = []

    try:
        with sync_playwright() as pw:
            browser = pw.chromium.launch(
                headless=True,
                args=["--hide-scrollbars", "--force-device-scale-factor=1"],
            )
            try:
                for (w, h) in viewports:
                    viewport_errors: list[str] = []
                    metrics: dict | None = None
                    page = browser.new_page(
                        viewport={"width": w, "height": h},
                        device_scale_factor=1,
                        reduced_motion="reduce",
                    )
                    page.on(
                        "console",
                        lambda msg, bucket=viewport_errors, ww=w: bucket.append(
                            f"w{w}: console.{msg.type}: {msg.text}"
                        )
                        if msg.type == "error"
                        else None,
                    )
                    page.on(
                        "pageerror",
                        lambda exc, bucket=viewport_errors, ww=w: bucket.append(f"w{ww}: pageerror: {exc}"),
                    )
                    try:
                        page.goto(url, wait_until="load", timeout=args.timeout_ms)
                    except PWTimeout:
                        result["warnings"].append(f"w{w}: 'load' timed out; measuring whatever rendered")
                    try:
                        page.wait_for_load_state("networkidle", timeout=3000)
                    except Exception:
                        pass
                    if css:
                        page.add_style_tag(content=css)
                        try:
                            page.evaluate("() => document.fonts.ready.then(() => document.fonts.size)")
                        except Exception:
                            pass
                    if args.settle_ms > 0:
                        page.wait_for_timeout(args.settle_ms)
                    try:
                        metrics = page.evaluate(MEASURE_JS, args.tolerance)
                    except Exception as exc:  # pragma: no cover - defensive
                        result["warnings"].append(f"w{w}: measurement failed: {exc!r}")
                    finally:
                        result["console_errors"].extend(viewport_errors)
                        warnings = result["warnings"]
                        page.close()
                    if metrics is None:
                        result["viewports"].append(
                            {"width": w, "height": 0, "horizontal_overflow": False, "overflow_px": 0,
                             "error": "measurement failed", "sections": [], "texts": [],
                             "fonts_used": [], "overflowing_elements": []}
                        )
                    else:
                        if not css:
                            metrics.pop("fonts_loaded", None)
                        result["viewports"].append(metrics)
                result["ok"] = all("error" not in vp for vp in result["viewports"])
            finally:
                browser.close()
    except Exception as exc:
        errors.append(f"{type(exc).__name__}: {exc}")
        result["warnings"].extend(errors)

    return result, (0 if result["ok"] else 2)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        prog="browser_verify.py",
        description="Measure real rendered geometry of a local HTML file with headless Chromium.",
    )
    ap.add_argument("--html", required=True, help="path to the local HTML file to measure")
    ap.add_argument(
        "--viewport",
        default=DEFAULT_VIEWPORTS,
        help="comma-separated widths, or WIDTHxHEIGHT (default: %s)" % DEFAULT_VIEWPORTS,
    )
    ap.add_argument(
        "--viewport-height",
        type=int,
        default=DEFAULT_VIEWPORT_HEIGHT,
        help="viewport height for width-only entries (default: %d)" % DEFAULT_VIEWPORT_HEIGHT,
    )
    ap.add_argument(
        "--fonts-dir",
        action="append",
        default=[],
        help="directory of local font files to register via @font-face (repeatable)",
    )
    ap.add_argument("--json", help="also write the JSON report to this path")
    ap.add_argument(
        "--tolerance",
        type=float,
        default=1.0,
        help="px tolerance for section containment (default: 1.0)",
    )
    ap.add_argument("--settle-ms", type=int, default=150, help="extra settle wait per viewport")
    ap.add_argument("--timeout-ms", type=int, default=30000, help="navigation timeout per viewport")
    args = ap.parse_args(argv)

    result, code = run(args)

    payload = json.dumps(result, indent=2, ensure_ascii=False, sort_keys=False)
    sys.stdout.write(payload + "\n")
    if args.json:
        with open(args.json, "w", encoding="utf-8") as fh:
            fh.write(payload + "\n")
    if result["warnings"]:
        for w in result["warnings"]:
            sys.stderr.write("warning: %s\n" % w)
    return code


if __name__ == "__main__":
    sys.exit(main())
