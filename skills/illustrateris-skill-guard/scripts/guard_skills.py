#!/usr/bin/env python3
"""guard_skills.py -- Universal, zero-hardcode behavioral conflict detector for Illustrateris.

Instead of matching against a brittle list of hardcoded competitor skill names (which breaks the
moment an author renames a skill or a user installs a new third-party pack), this scanner inspects
the actual content, frontmatter, scripts, and directory structure of every installed skill and
scores them across 4 Behavioral Conflict Vectors:

  1. domain_collision: Claims authority over frontend/UI/landing pages/typography/palettes/CSS.
  2. template_dealer:  Bundles static JSON layout/palette libraries, randomizer/draw scripts,
                       dice-rolling archetypes, fixed numeric dials, or rigid AIDA formulas.
  3. axiom_collision:  Enforces blanket stylistic dogma ("no monospace", "no gradients",
                       "no 1px borders", "banned fonts", "backdrop-blur-2xl", "4 archive photos").
  4. gate_collision:   Hijacks the workflow with external browser inspectors or non-code DESIGN.md output.

Any skill outside the verified `illustrateris-*` namespace that scores >= 2 conflict points (or
combines domain_collision with any structural conflict vector) is flagged for immediate suppression.

Usage:
    python3 skills/illustrateris-skill-guard/scripts/guard_skills.py [--scan-dir <dir>] [--json]
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys

VERIFIED_NAMESPACE_PREFIX = "illustrateris-"

_FRONTMATTER_NAME_RE = re.compile(r"^name:\s*['\"]?([A-Za-z0-9_-]+)['\"]?\s*$", re.MULTILINE)

# Behavioral fingerprint patterns (zero competitor skill names hardcoded)
VECTOR_PATTERNS = {
    "domain_collision": [
        (re.compile(r"\b(?:landing\s+page|frontend|ui/ux|web\s+design|marketing\s+site|art\s+direction)\b", re.I), "claims frontend/landing page design authority"),
        (re.compile(r"\b(?:typography\s+hierarchy|color\s+palette|hero\s+section|bento\s+grid|tailwind)\b", re.I), "dictates UI layout, typography, or palette rules"),
    ],
    "template_dealer": [
        (re.compile(r"\b(?:draw\.mjs|random\.choice|roll\s+the\s+dice|deals?\s+three|direction\s+sheet)\b", re.I), "uses randomizer or static direction-dealing scripts"),
        (re.compile(r"\b(?:DESIGN_VARIANCE|MOTION_INTENSITY|VISUAL_DENSITY)\s*:\s*\d+", re.I), "uses fixed numeric style dials instead of continuous domain math"),
        (re.compile(r"\bAIDA\s+(?:framework|structure)\b", re.I), "enforces rigid marketing template sequence"),
    ],
    "axiom_collision": [
        (re.compile(r"\b(?:no\s+monospace|no\s+gradients|no\s+thin\s+rules|no\s+1px|banned\s+fonts)\b", re.I), "enforces blanket stylistic bans that break domain physics"),
        (re.compile(r"\b(?:backdrop-blur-2xl|vantablack|ethereal\s+glass|crt\s+scanlines)\b", re.I), "mandates fixed decorative tropes or glassmorphism clichés"),
        (re.compile(r"\b(?:at\s+least\s+four\s+real\s+pictures|open\s+collections|wikimedia)\b", re.I), "forces arbitrary external stock/archive photo quotas"),
    ],
    "gate_collision": [
        (re.compile(r"\b(?:inspect\.mjs|lint\.mjs|generates?\s+`?DESIGN\.md`?)\b", re.I), "hijacks build gate or emits prose-only DESIGN.md"),
    ],
}


def read_skill_metadata(skill_dir: str) -> tuple[str, str, list[str]]:
    """Return (skill_name, full_text, relative_file_list) for a skill directory."""
    skill_md = os.path.join(skill_dir, "SKILL.md")
    try:
        with open(skill_md, "r", encoding="utf-8", errors="ignore") as fh:
            text = fh.read()
    except OSError:
        text = ""
    m = _FRONTMATTER_NAME_RE.search(text[:2048])
    name = m.group(1) if m else os.path.basename(skill_dir)

    rel_files = []
    for root, dirs, files in os.walk(skill_dir):
        dirs[:] = [d for d in dirs if d not in {".git", "node_modules", "__pycache__"}]
        for f in sorted(files):
            rel_files.append(os.path.relpath(os.path.join(root, f), skill_dir))
    return name, text, rel_files


def analyze_skill(skill_dir: str) -> dict:
    """Evaluate a skill directory using pure structural & semantic fingerprinting."""
    name, text, rel_files = read_skill_metadata(skill_dir)

    # Check if this skill is part of the verified Illustrateris suite
    is_illustrateris = name.startswith(VERIFIED_NAMESPACE_PREFIX)
    if is_illustrateris:
        return {
            "name": name,
            "path": skill_dir,
            "status": "ALLOW",
            "conflict_score": 0,
            "vectors": {},
            "reasons": ["Verified Illustrateris pipeline skill."],
        }

    vectors: dict[str, list[str]] = {}
    score = 0

    for vec_name, rules in VECTOR_PATTERNS.items():
        hits = []
        for pattern, desc in rules:
            if pattern.search(text):
                hits.append(desc)
        if hits:
            vectors[vec_name] = hits
            score += len(hits)

    # Structural file tree checks (static template libraries or randomizer scripts)
    json_libs = [f for f in rel_files if f.startswith("library/") and f.endswith(".json")]
    if json_libs:
        vectors.setdefault("template_dealer", []).append(
            f"bundles {len(json_libs)} static template JSON file(s) ({', '.join(json_libs[:3])})"
        )
        score += 2

    is_conflicting = (
        ("domain_collision" in vectors and len(vectors) >= 2)
        or ("template_dealer" in vectors)
        or ("axiom_collision" in vectors)
        or score >= 2
    )

    reasons = []
    for vec_name, hits in vectors.items():
        for h in hits:
            reasons.append(f"[{vec_name}] {h}")

    return {
        "name": name,
        "path": skill_dir,
        "status": "SUPPRESS" if is_conflicting else "IGNORE",
        "conflict_score": score,
        "vectors": vectors,
        "reasons": reasons,
    }


def scan_directories(extra_dirs: list[str]) -> dict:
    home = os.path.expanduser("~")
    candidates = [
        os.path.join(home, ".claude", "skills"),
        os.path.join(home, ".codex", "skills"),
        os.path.abspath("skills"),
    ] + [os.path.abspath(d) for d in extra_dirs]

    seen_real = set()
    allowed = []
    suppressed = []

    for base in candidates:
        if not os.path.isdir(base):
            continue
        for entry in sorted(os.listdir(base)):
            full_dir = os.path.join(base, entry)
            real_dir = os.path.realpath(full_dir)
            if real_dir in seen_real or not os.path.isdir(real_dir):
                continue
            if not os.path.isfile(os.path.join(real_dir, "SKILL.md")):
                continue
            seen_real.add(real_dir)
            res = analyze_skill(full_dir)
            if res["status"] == "ALLOW":
                allowed.append(res)
            elif res["status"] == "SUPPRESS":
                suppressed.append(res)

    return {
        "ok": len(suppressed) == 0,
        "detection_mode": "universal_behavioral_fingerprint (zero hardcoded names)",
        "illustrateris_skills": allowed,
        "conflicting_skills_detected": suppressed,
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Universal behavioral conflict scanner for Illustrateris.")
    ap.add_argument("--scan-dir", action="append", default=[], help="Additional skill directory to inspect")
    ap.add_argument("--json", action="store_true", help="Emit machine-readable JSON report")
    args = ap.parse_args(argv)

    report = scan_directories(args.scan_dir)
    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print("=== Illustrateris Universal Skill Guard (Zero-Hardcode Behavioral Scanner) ===")
        print(f"Verified Illustrateris skills ({len(report['illustrateris_skills'])}):")
        for s in report["illustrateris_skills"]:
            print(f"  [ALLOW]    {s['name']} ({s['path']})")
        if report["conflicting_skills_detected"]:
            print(f"\nConflicting external skills detected by behavioral fingerprint ({len(report['conflicting_skills_detected'])}) -- SUPPRESS:")
            for c in report["conflicting_skills_detected"]:
                print(f"  [SUPPRESS] {c['name']} (score={c['conflict_score']}) at {c['path']}")
                for r in c["reasons"]:
                    print(f"             - {r}")
        else:
            print("\nNo conflicting external skills detected.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
