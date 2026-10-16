#!/usr/bin/env python3
# @generated-file: no -- this file is hand-written tooling; its OUTPUT is generated.
"""Extract real per-glyph advance widths and OpenType GPOS kerning from TrueType/OpenType fonts and emit Rust.

Dependency-free (stdlib only, no fonttools/pip). Parses the sfnt container directly:

    head  -> unitsPerEm, indexToLocFormat
    maxp  -> numGlyphs
    hhea  -> numberOfHMetrics
    hmtx  -> advance widths (monospaced tail handled)
    cmap  -> character -> glyph id (formats 4 and 12)
    GPOS  -> PairPos kerning adjustments (formats 1 and 2, plus LookupType 9 extension)

For each (family, weight) pair it emits a sorted table of
``unicode codepoint -> advance width in em units`` rounded to 4 decimals,
covering ASCII 0x20-0x7E, Latin-1 Supplement 0xA0-0xFF, Latin Extended-A,
Cyrillic (Russian + Kazakh/Extended), and common typographic/UI codepoints,
plus GPOS kerning tables and East Asian Wide / CJK fullwidth fallback.

Usage:
    python3 tools/gen_font_metrics.py --fonts fonts --out src/text/metrics_data.rs

The output is deterministic: stable ordering, no timestamps, no dict iteration
order leakage.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import struct
import sys
from bisect import bisect_left

# --------------------------------------------------------------------------
# Configuration: the bundled reference fonts.
# --------------------------------------------------------------------------

SANS_NAME = "Inter"
SERIF_NAME = "Source Serif 4"
MONO_NAME = "JetBrains Mono"

# (family, weight) -> relative path under --fonts
FONT_FILES = {
    ("sans", 400): "sans/Inter-Regular.ttf",
    ("sans", 700): "sans/Inter-Bold.ttf",
    ("serif", 400): "serif/SourceSerif4-Regular.ttf",
    ("serif", 400): "serif/SourceSerif4-Regular.ttf",
    ("serif", 700): "serif/SourceSerif4-Bold.ttf",
    ("mono", 400): "mono/JetBrainsMono-Regular.ttf",
    ("mono", 700): "mono/JetBrainsMono-Bold.ttf",
}

FAMILY_ORDER = ("sans", "serif", "mono")

# Fallback glyph used for "codepoint not in the font": the question mark.
DEFAULT_CHAR = 0x3F

CSS_STACKS = {
    "sans": "Inter,ui-sans-serif,system-ui,-apple-system,Segoe_UI,Roboto,"
    "Helvetica,Arial,sans-serif",
    "serif": "Source_Serif_4,Iowan_Old_Style,Palatino_Linotype,Palatino,"
    "Georgia,Cambria,serif",
    "mono": "JetBrains_Mono,ui-monospace,SFMono-Regular,Menlo,Consolas,"
    "Liberation_Mono,monospace",
}


def target_codepoints():
    """Codepoints the generated tables must cover, ascending and de-duplicated."""
    cps = set(range(0x20, 0x7F))  # ASCII printable
    cps |= set(range(0xA0, 0x100))  # Latin-1 Supplement
    cps |= set(range(0x0100, 0x0180))  # Latin Extended-A
    cps |= set(range(0x0400, 0x0460))  # Cyrillic core (Russian, Ukrainian, Belarusian, etc.)
    cps |= {
        # Kazakh & Extended Cyrillic
        0x0492, 0x0493, 0x049A, 0x049B, 0x04A2, 0x04A3,
        0x04AE, 0x04AF, 0x04B0, 0x04B1, 0x04BA, 0x04BB,
        0x04D8, 0x04D9, 0x04E8, 0x04E9,
        # Typographic punctuation, currencies, math & directional UI glyphs
        0x2013,  # en dash
        0x2014,  # em dash
        0x2018,  # left single quotation mark
        0x2019,  # right single quotation mark
        0x201C,  # left double quotation mark
        0x201D,  # right double quotation mark
        0x2022,  # bullet
        0x2026,  # horizontal ellipsis
        0x00A0,  # no-break space
        0x2039,  # single left-pointing angle quotation mark
        0x203A,  # single right-pointing angle quotation mark
        0x00AD,  # soft hyphen
        0x20AC,  # euro sign
        0x20B8,  # tenge sign
        0x20BD,  # ruble sign
        0x20B9,  # indian rupee sign
        0x2122,  # trade mark sign
        0x2190,  # leftwards arrow
        0x2191,  # upwards arrow
        0x2192,  # rightwards arrow
        0x2193,  # downwards arrow
        0x2197,  # north east arrow
        0x2198,  # south east arrow
        0x2212,  # minus sign
        0x221E,  # infinity
        0x2260,  # not equal to
        0x2264,  # less-than or equal to
        0x2265,  # greater-than or equal to
        0x00B0,  # degree sign
        0x00B1,  # plus-minus sign
        0x00D7,  # multiplication sign
    }
    return sorted(cps)


def kern_candidate_codepoints():
    """Codepoints checked for pairwise GPOS kerning (Latin + Cyrillic letters & punctuation)."""
    cps = []
    for cp in range(0x21, 0x7F):
        ch = chr(cp)
        if ch.isalpha() or ch in ".,:;!?'-/()\"":
            cps.append(cp)
    for cp in list(range(0x0410, 0x0450)) + [0x0401, 0x0451]:
        cps.append(cp)
    for cp in (0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2026):
        cps.append(cp)
    return sorted(set(cps))


# --------------------------------------------------------------------------
# sfnt parsing
# --------------------------------------------------------------------------


class FontError(Exception):
    pass


def _u16(b, o):
    return struct.unpack_from(">H", b, o)[0]


def _i16(b, o):
    return struct.unpack_from(">h", b, o)[0]


def _u32(b, o):
    return struct.unpack_from(">I", b, o)[0]


def read_table_directory(data):
    if len(data) < 12:
        raise FontError("file too short for an sfnt header")
    tag = data[0:4]
    if tag == b"ttcf":
        raise FontError("TrueType Collections are not supported; use a single-face file")
    if tag not in (b"\x00\x01\x00\x00", b"OTTO", b"true", b"typ1"):
        raise FontError("unrecognised sfnt version %r" % (tag,))
    num_tables = _u16(data, 4)
    tables = {}
    for i in range(num_tables):
        rec = 12 + i * 16
        if rec + 16 > len(data):
            raise FontError("truncated table directory")
        name = data[rec : rec + 4]
        offset = _u32(data, rec + 8)
        length = _u32(data, rec + 12)
        tables[name] = (offset, length)
    return tables


def units_per_em(data, tables):
    off, _ = tables[b"head"]
    upm = _u16(data, off + 18)
    if upm == 0:
        raise FontError("head.unitsPerEm is zero")
    return upm


def num_glyphs(data, tables):
    off, _ = tables[b"maxp"]
    return _u16(data, off + 4)


def advance_widths(data, tables):
    """Full per-glyph advance width array, indexed by glyph id."""
    if b"hmtx" not in tables or b"hhea" not in tables:
        raise FontError("missing hmtx/hhea")
    hhea = tables[b"hhea"][0]
    n_metrics = _u16(data, hhea + 34)
    if n_metrics == 0:
        raise FontError("hhea.numberOfHMetrics is zero")
    total = num_glyphs(data, tables)
    hmtx = tables[b"hmtx"][0]
    out = []
    for i in range(n_metrics):
        out.append(_u16(data, hmtx + i * 4))
    last = out[-1]
    # Glyphs beyond numberOfHMetrics share the final advance width.
    while len(out) < total:
        out.append(last)
    return out


def _pick_cmap_subtable(data, tables):
    if b"cmap" not in tables:
        raise FontError("missing cmap")
    base = tables[b"cmap"][0]
    n = _u16(data, base + 2)
    best_off = None
    best_score = -1
    for i in range(n):
        rec = base + 4 + i * 8
        pid, eid, off = struct.unpack_from(">HHI", data, rec)
        sub = base + off
        fmt = _u16(data, sub)
        if fmt == 12:
            score = 6 if (pid, eid) in ((3, 10), (0, 4), (0, 6)) else 5
        elif fmt == 4:
            score = 4 if (pid, eid) in ((3, 1), (0, 3)) else 3
        else:
            score = -1
        if score > best_score:
            best_score = score
            best_off = (sub, fmt)
    if best_off is None:
        raise FontError("no usable cmap subtable (format 4 or 12)")
    return best_off


def _cmap4_ranges(data, off):
    seg_x2 = _u16(data, off + 6)
    seg = seg_x2 // 2
    ends_off = off + 14
    starts_off = ends_off + seg_x2 + 2
    deltas_off = starts_off + seg_x2
    ranges_off = deltas_off + seg_x2
    ends = [_u16(data, ends_off + i * 2) for i in range(seg)]
    starts = [_u16(data, starts_off + i * 2) for i in range(seg)]
    deltas = [_i16(data, deltas_off + i * 2) for i in range(seg)]
    ranges = [_u16(data, ranges_off + i * 2) for i in range(seg)]

    def lookup(cp):
        i = bisect_left(ends, cp)
        if i >= seg:
            return None
        if cp < starts[i]:
            return None
        if ranges[i] == 0:
            gid = (cp + deltas[i]) & 0xFFFF
        else:
            idx = ranges_off + i * 2 + ranges[i] + (cp - starts[i]) * 2
            if idx + 2 > len(data):
                return None
            gid = _u16(data, idx)
            if gid == 0:
                return None
            gid = (gid + deltas[i]) & 0xFFFF
        return gid if gid else None

    return lookup


def _cmap12_ranges(data, off):
    n_groups = _u32(data, off + 12)
    starts = []
    ends = []
    gids = []
    p = off + 16
    for _ in range(n_groups):
        s, e, g = struct.unpack_from(">III", data, p)
        p += 12
        starts.append(s)
        ends.append(e)
        gids.append(g)

    def lookup(cp):
        i = bisect_left(ends, cp)
        if i >= len(starts) or cp < starts[i]:
            return None
        gid = gids[i] + (cp - starts[i])
        return gid if gid else None

    return lookup


def cmap_lookup(data, tables):
    sub, fmt = _pick_cmap_subtable(data, tables)
    if fmt == 4:
        return _cmap4_ranges(data, sub)
    if fmt == 12:
        return _cmap12_ranges(data, sub)
    raise FontError("unsupported cmap format %d" % fmt)


# --------------------------------------------------------------------------
# OpenType GPOS PairPos Kerning Extraction
# --------------------------------------------------------------------------


def _parse_coverage(data, off):
    fmt = _u16(data, off)
    if fmt == 1:
        cnt = _u16(data, off + 2)
        return list(struct.unpack_from(f">{cnt}H", data, off + 4))
    if fmt == 2:
        cnt = _u16(data, off + 2)
        out = []
        for i in range(cnt):
            s, e, _ = struct.unpack_from(">HHH", data, off + 4 + i * 6)
            out.extend(range(s, e + 1))
        return out
    return []


def _parse_class_def(data, off):
    fmt = _u16(data, off)
    m = {}
    if fmt == 1:
        start_gid, cnt = struct.unpack_from(">HH", data, off + 2)
        vals = struct.unpack_from(f">{cnt}H", data, off + 6)
        for i, v in enumerate(vals):
            if v != 0:
                m[start_gid + i] = v
    elif fmt == 2:
        cnt = _u16(data, off + 2)
        for i in range(cnt):
            s, e, c = struct.unpack_from(">HHH", data, off + 4 + i * 6)
            if c != 0:
                for gid in range(s, e + 1):
                    m[gid] = c
    return m


def _value_record_len(vf):
    return bin(vf).count("1") * 2


def _read_x_advance(data, off, vf):
    if not (vf & 0x0004):
        return 0
    pos = off
    if vf & 0x0001:
        pos += 2
    if vf & 0x0002:
        pos += 2
    return _i16(data, pos)


def extract_kerning_pairs(data, tables, lookup, upm, kern_cps, threshold_em=0.025):
    """Extract significant (left_cp, right_cp) -> kern_em pairs from GPOS 'kern' lookups."""
    if b"GPOS" not in tables:
        return []
    gpos = tables[b"GPOS"][0]
    fl = gpos + _u16(data, gpos + 6)
    ll = gpos + _u16(data, gpos + 8)
    fcnt = _u16(data, fl)
    kern_lookups = []
    for i in range(fcnt):
        tag = data[fl + 2 + i * 6 : fl + 6 + i * 6]
        foff = fl + _u16(data, fl + 6 + i * 6)
        if tag == b"kern":
            lcnt = _u16(data, foff + 2)
            for idx in struct.unpack_from(f">{lcnt}H", data, foff + 4):
                if idx not in kern_lookups:
                    kern_lookups.append(idx)
    if not kern_lookups:
        return []

    cp_to_gid = {}
    for cp in kern_cps:
        gid = lookup(cp)
        if gid is not None:
            cp_to_gid[cp] = gid
    active_gids = set(cp_to_gid.values())

    subtables = []
    for lidx in kern_lookups:
        loff = ll + _u16(data, ll + 2 + lidx * 2)
        ltype, _, scnt = struct.unpack_from(">HHH", data, loff)
        for s in range(scnt):
            soff = loff + _u16(data, loff + 6 + s * 2)
            st = ltype
            if st == 9:
                _, st, ext_off = struct.unpack_from(">HHI", data, soff)
                soff = soff + ext_off
            if st != 2:
                continue
            fmt, cov_off, vf1, vf2 = struct.unpack_from(">HHHH", data, soff)
            v1_len = _value_record_len(vf1)
            v2_len = _value_record_len(vf2)
            cov_list = _parse_coverage(data, soff + cov_off)
            cov_set = set(cov_list)
            if fmt == 1:
                pair_map = {}
                for i, g1 in enumerate(cov_list):
                    if g1 not in active_gids:
                        continue
                    ps_off = soff + _u16(data, soff + 10 + i * 2)
                    p_cnt = _u16(data, ps_off)
                    rec_size = 2 + v1_len + v2_len
                    for j in range(p_cnt):
                        ro = ps_off + 2 + j * rec_size
                        g2 = _u16(data, ro)
                        if g2 in active_gids:
                            xa = _read_x_advance(data, ro + 2, vf1)
                            if xa != 0:
                                pair_map[(g1, g2)] = xa
                subtables.append((1, cov_set, pair_map))
            elif fmt == 2:
                cd1_off, cd2_off, c1_cnt, c2_cnt = struct.unpack_from(">HHHH", data, soff + 8)
                cd1 = _parse_class_def(data, soff + cd1_off)
                cd2 = _parse_class_def(data, soff + cd2_off)
                rec_size = v1_len + v2_len
                subtables.append((2, cov_set, cd1, cd2, soff + 16, c1_cnt, c2_cnt, rec_size, vf1))

    pairs = []
    for cp1 in kern_cps:
        g1 = cp_to_gid.get(cp1)
        if g1 is None:
            continue
        for cp2 in kern_cps:
            g2 = cp_to_gid.get(cp2)
            if g2 is None:
                continue
            val = 0
            for st in subtables:
                if st[0] == 1:
                    _, cov_set, pair_map = st
                    if g1 in cov_set and (g1, g2) in pair_map:
                        val = pair_map[(g1, g2)]
                        break
                elif st[0] == 2:
                    _, cov_set, cd1, cd2, base, c1_cnt, c2_cnt, rec_size, vf1 = st
                    if g1 in cov_set:
                        c1 = cd1.get(g1, 0)
                        c2 = cd2.get(g2, 0)
                        if c1 < c1_cnt and c2 < c2_cnt:
                            v = _read_x_advance(data, base + (c1 * c2_cnt + c2) * rec_size, vf1)
                            if v != 0:
                                val = v
                                break
            if val != 0:
                em = round(val / upm, 4)
                if abs(em) >= threshold_em:
                    packed = ((cp1 & 0xFFFF) << 16) | (cp2 & 0xFFFF)
                    pairs.append((packed, em))
    pairs.sort(key=lambda x: x[0])
    return pairs


# --------------------------------------------------------------------------
# Metrics extraction
# --------------------------------------------------------------------------


def _fmt_float(x):
    """Deterministic 4-decimal Rust literal with trailing zeros trimmed."""
    s = "%.4f" % x
    if "." in s:
        s = s.rstrip("0")
        if s.endswith("."):
            s = s + "0"
    if s == "-0.0":
        s = "0.0"
    return s


def extract(path, targets, kern_cps):
    """Return (entries, kern_pairs, default, average, upm, sha256) for one font file."""
    with open(path, "rb") as fh:
        data = fh.read()
    sha = hashlib.sha256(data).hexdigest()
    tables = read_table_directory(data)
    for req in (b"head", b"maxp", b"hhea", b"hmtx", b"cmap"):
        if req not in tables:
            raise FontError("%s: missing required table %s" % (path, req.decode()))
    upm = units_per_em(data, tables)
    advances = advance_widths(data, tables)
    lookup = cmap_lookup(data, tables)

    def advance_of(cp):
        gid = lookup(cp)
        if gid is None or gid >= len(advances):
            return None
        a = advances[gid]
        return a if a > 0 else None

    default_raw = advance_of(DEFAULT_CHAR)
    if default_raw is None:
        seen = [advance_of(cp) for cp in targets]
        seen = [v for v in seen if v]
        default_raw = int(round(sum(seen) / len(seen))) if seen else max(upm // 2, 1)

    entries = []
    for cp in targets:
        a = advance_of(cp)
        entries.append((cp, a if a is not None else default_raw))

    # Keep average_em anchored to ASCII + Latin-1 range so existing layout expectations stay rock-solid
    core_entries = [a for cp, a in entries if cp <= 0xFF or cp >= 0x2000]
    default_em = round(default_raw / upm, 4)
    average_em = round(sum(core_entries) / len(core_entries) / upm, 4)
    entries_em = [(cp, round(a / upm, 4)) for cp, a in entries]
    kern_pairs = extract_kerning_pairs(data, tables, lookup, upm, kern_cps)
    return entries_em, kern_pairs, default_em, average_em, upm, sha


# --------------------------------------------------------------------------
# Rust emission
# --------------------------------------------------------------------------


def const_name(family, weight):
    return "%s_%d" % (family.upper(), weight)


def emit_table(out, family, weight, entries):
    out.append("/// `unicode codepoint -> advance width in em` for %s at weight %d." % (family, weight))
    out.append("pub static %s: &[(u32, f64)] = &[" % const_name(family, weight))
    line = "   "
    for cp, em in entries:
        item = " (0x%04X, %s)," % (cp, _fmt_float(em))
        if len(line) + len(item) > 98:
            out.append(line)
            line = "   "
        line += item
    if line.strip():
        out.append(line)
    out.append("];")
    out.append("")


def emit_kern_table(out, family, weight, pairs):
    out.append("/// `((left_cp << 16) | right_cp) -> kerning adjustment in em` for %s at weight %d." % (family, weight))
    out.append("pub static KERN_%s: &[(u32, f64)] = &[" % const_name(family, weight))
    line = "   "
    for packed, em in pairs:
        item = " (0x%08X, %s)," % (packed, _fmt_float(em))
        if len(line) + len(item) > 98:
            out.append(line)
            line = "   "
        line += item
    if line.strip():
        out.append(line)
    out.append("];")
    out.append("")


def generate(fonts_dir, targets, kern_cps):
    """Return the full Rust source text."""
    families = {}
    for (family, weight), rel in sorted(FONT_FILES.items()):
        path = os.path.join(fonts_dir, rel)
        if not os.path.isfile(path):
            raise SystemExit("missing font file: %s" % path)
        entries, kern_pairs, default_em, average_em, upm, sha = extract(path, targets, kern_cps)
        families.setdefault(family, {})[weight] = {
            "entries": entries,
            "kern": kern_pairs,
            "default": default_em,
            "average": average_em,
            "upm": upm,
            "sha256": sha,
            "file": rel,
        }

    out = []
    out.append("// @generated by tools/gen_font_metrics.py -- do not edit by hand")
    out.append("//! Per-glyph advance-width and OpenType GPOS kerning metrics extracted from the bundled fonts.")
    out.append("//!")
    out.append("//! Each table maps a Unicode codepoint to the glyph's advance width as a")
    out.append("//! fraction of the em box (advance / unitsPerEm), rounded to 4 decimals.")
    out.append("//! Codepoints the font does not cover fall back to East Asian Wide (1.0em)")
    out.append("//! for CJK/ideographs or to that font's default advance (`?` glyph).")
    out.append("//!")
    out.append("//! Sources (see fonts/SOURCES.md):")
    for family in FAMILY_ORDER:
        for weight in (400, 700):
            info = families[family][weight]
            out.append(
                "//!   %-5s %d  %-38s upm=%-5d sha256=%s"
                % (family, weight, info["file"], info["upm"], info["sha256"][:16])
            )
    out.append("")

    out.append("/// Font family selector for the bundled reference metrics.")
    out.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    out.append("pub enum Family {")
    out.append("    /// UI / body sans (Inter).")
    out.append("    Sans,")
    out.append("    /// Long-form editorial serif (Source Serif 4).")
    out.append("    Serif,")
    out.append("    /// Fixed-width / code (JetBrains Mono).")
    out.append("    Mono,")
    out.append("}")
    out.append("")

    for family in FAMILY_ORDER:
        for weight in (400, 700):
            emit_table(out, family, weight, families[family][weight]["entries"])

    for family in FAMILY_ORDER:
        for weight in (400, 700):
            emit_kern_table(out, family, weight, families[family][weight]["kern"])

    out.append("/// Default advance (em) for codepoints absent from the tables, per weight.")
    for family in FAMILY_ORDER:
        for weight in (400, 700):
            info = families[family][weight]
            out.append(
                "const DEFAULT_%s: f64 = %s;"
                % (const_name(family, weight), _fmt_float(info["default"]))
            )
    out.append("")
    out.append("/// Mean advance (em) over the mapped ranges at weight 400.")
    for family in FAMILY_ORDER:
        info = families[family][400]
        out.append(
            "const AVG_%s: f64 = %s;"
            % (const_name(family, 400), _fmt_float(info["average"]))
        )
    out.append("")

    out.append(
        """/// A static codepoint-keyed table of em advances (or kerning pairs) for one embedded face.
type GlyphTable = &'static [(u32, f64)];

fn tables(family: Family) -> (GlyphTable, GlyphTable) {
    match family {
        Family::Sans => (SANS_400, SANS_700),
        Family::Serif => (SERIF_400, SERIF_700),
        Family::Mono => (MONO_400, MONO_700),
    }
}

fn kern_tables(family: Family) -> (GlyphTable, GlyphTable) {
    match family {
        Family::Sans => (KERN_SANS_400, KERN_SANS_700),
        Family::Serif => (KERN_SERIF_400, KERN_SERIF_700),
        Family::Mono => (KERN_MONO_400, KERN_MONO_700),
    }
}

/// Binary search over a table sorted by key.
fn lookup(table: &[(u32, f64)], key: u32) -> Option<f64> {
    table
        .binary_search_by_key(&key, |&(k, _)| k)
        .ok()
        .map(|i| table[i].1)
}

/// Linear blend between the embedded 400 and 700 tables. Weights are clamped to
/// `400..=700`, so lighter/heavier requests reuse the nearest embedded instance.
fn blend(w400: f64, w700: f64, weight: u16) -> f64 {
    let t = (weight.clamp(400, 700) as f64 - 400.0) / 300.0;
    w400 + (w700 - w400) * t
}

/// True for East Asian Wide / Fullwidth / CJK / Emoji codepoints that occupy a full 1.0em cell.
pub fn is_wide_codepoint(cp: u32) -> bool {
    matches!(
        cp,
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3040..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7FF
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF01..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x2FA1F
    )
}

/// Advance width of `ch` in em units (fraction of font size) for the family at the given
/// CSS font weight, linearly interpolated between the embedded 400 and 700 tables.
pub fn advance_em(family: Family, ch: char, weight: u16) -> f64 {
    let cp = ch as u32;
    let (t400, t700) = tables(family);
    match (lookup(t400, cp), lookup(t700, cp)) {
        (Some(a), Some(b)) => blend(a, b, weight),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => {
            if is_wide_codepoint(cp) {
                1.0
            } else {
                default_advance_em(family, weight)
            }
        }
    }
}

/// Pairwise OpenType GPOS kerning adjustment in em units between `left` and `right`.
pub fn kern_em(family: Family, left: char, right: char, weight: u16) -> f64 {
    let l = left as u32;
    let r = right as u32;
    if l > 0xFFFF || r > 0xFFFF {
        return 0.0;
    }
    let key = (l << 16) | r;
    let (k400, k700) = kern_tables(family);
    match (lookup(k400, key), lookup(k700, key)) {
        (Some(a), Some(b)) => blend(a, b, weight),
        (Some(a), None) => blend(a, 0.0, weight),
        (None, Some(b)) => blend(0.0, b, weight),
        (None, None) => 0.0,
    }
}

/// Advance to use for a non-wide codepoint not present in the tables.
pub fn default_advance_em(family: Family, weight: u16) -> f64 {
    let (d400, d700) = match family {
        Family::Sans => (DEFAULT_SANS_400, DEFAULT_SANS_700),
        Family::Serif => (DEFAULT_SERIF_400, DEFAULT_SERIF_700),
        Family::Mono => (DEFAULT_MONO_400, DEFAULT_MONO_700),
    };
    blend(d400, d700, weight)
}

/// One representative advance per family at weight 400 (kept for backwards-compatible averages).
pub fn average_advance_em(family: Family) -> f64 {
    match family {
        Family::Sans => AVG_SANS_400,
        Family::Serif => AVG_SERIF_400,
        Family::Mono => AVG_MONO_400,
    }
}
"""
    )

    out.append("/// The human-facing font-family CSS stack for each family, unquoted and")
    out.append("/// comma-separated with no literal spaces (safe inside Tailwind arbitrary values).")
    out.append("pub fn css_stack(family: Family) -> &'static str {")
    out.append("    match family {")
    out.append('        Family::Sans => "%s",' % CSS_STACKS["sans"])
    out.append('        Family::Serif => "%s",' % CSS_STACKS["serif"])
    out.append('        Family::Mono => "%s",' % CSS_STACKS["mono"])
    out.append("    }")
    out.append("}")
    out.append("")

    out.append(
        """#[cfg(test)]
mod tests {
    use super::*;

    fn weights() -> [u16; 3] {
        [400, 550, 700]
    }

    #[test]
    fn space_is_wide_enough() {
        assert!(advance_em(Family::Sans, ' ', 400) > 0.2, "space advance too small");
    }

    #[test]
    fn sans_is_proportional() {
        assert!(advance_em(Family::Sans, 'i', 400) < advance_em(Family::Sans, 'm', 400));
        assert!(advance_em(Family::Serif, 'i', 400) < advance_em(Family::Serif, 'm', 400));
    }

    #[test]
    fn cyrillic_and_wide_glyphs_are_measured() {
        assert!(advance_em(Family::Sans, 'Ж', 400) > advance_em(Family::Sans, 'І', 400));
        assert!(advance_em(Family::Serif, 'Ш', 700) > advance_em(Family::Serif, 'а', 700));
        assert!((advance_em(Family::Sans, '漢', 400) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn kerning_tightens_classic_pairs() {
        assert!(kern_em(Family::Sans, 'A', 'V', 400) < -0.04);
        assert!(kern_em(Family::Sans, 'T', 'o', 400) < -0.04);
        assert_eq!(kern_em(Family::Mono, 'A', 'V', 400), 0.0);
    }

    #[test]
    fn mono_is_near_constant() {
        let i = advance_em(Family::Mono, 'i', 400);
        let m = advance_em(Family::Mono, 'm', 400);
        let w = advance_em(Family::Mono, 'W', 400);
        assert!((i - m).abs() < 0.001, "i={i} m={m}");
        assert!((m - w).abs() < 0.001, "m={m} W={w}");
    }

    #[test]
    fn serif_differs_from_sans() {
        assert!(advance_em(Family::Serif, 'm', 400) != advance_em(Family::Sans, 'm', 400));
    }

    #[test]
    fn interpolation_is_monotonic() {
        let [a, b, c] = weights();
        for family in [Family::Sans, Family::Serif, Family::Mono] {
            for cp in 0x20u32..=0xFF {
                let ch = char::from_u32(cp).unwrap();
                let (lo, mid, hi) = (
                    advance_em(family, ch, a),
                    advance_em(family, ch, b),
                    advance_em(family, ch, c),
                );
                let rising = lo <= hi;
                if rising {
                    assert!(lo <= mid && mid <= hi, "not rising at U+{cp:04X}");
                } else {
                    assert!(hi <= mid && mid <= lo, "not falling at U+{cp:04X}");
                }
            }
        }
    }

    #[test]
    fn every_ascii_char_is_positive() {
        for family in [Family::Sans, Family::Serif, Family::Mono] {
            for weight in weights() {
                for cp in 0x20u32..=0x7E {
                    let ch = char::from_u32(cp).unwrap();
                    assert!(
                        advance_em(family, ch, weight) > 0.0,
                        "non-positive advance at U+{cp:04X} for {family:?} @ {weight}"
                    );
                }
            }
        }
    }

    #[test]
    fn defaults_and_averages_are_sane() {
        for family in [Family::Sans, Family::Serif, Family::Mono] {
            assert!(default_advance_em(family, 400) > 0.0);
            assert!(default_advance_em(family, 700) >= default_advance_em(family, 400));
            let avg = average_advance_em(family);
            assert!((0.4..0.75).contains(&avg), "average {avg} out of range");
        }
    }

    #[test]
    fn css_stacks_have_no_spaces() {
        for family in [Family::Sans, Family::Serif, Family::Mono] {
            let s = css_stack(family);
            assert!(!s.contains(' '), "stack contains a space: {s}");
            assert!(!s.contains('"'), "stack is quoted: {s}");
            assert!(s.contains(','));
        }
    }
}
"""
    )
    return "\n".join(out)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--fonts", required=True, help="directory containing the bundled font files")
    ap.add_argument("--out", required=True, help="path of the Rust file to write")
    args = ap.parse_args(argv)

    targets = target_codepoints()
    kern_cps = kern_candidate_codepoints()
    src = generate(args.fonts, targets, kern_cps)
    if not src.endswith("\n"):
        src += "\n"

    out_dir = os.path.dirname(os.path.abspath(args.out))
    if out_dir:
        os.makedirs(out_dir, exist_ok=True)
    with open(args.out, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(src)
    sys.stderr.write("wrote %s (%d bytes, %d codepoints)\n" % (args.out, len(src.encode()), len(targets)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
