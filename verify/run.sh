#!/bin/sh
'exec' 'python3' "$0" "$@" # '''
"""verify/run.sh -- convenience runner around browser_verify.py.

Valid as POSIX sh (line 2 re-execs python3) *and* as Python, so both of these work:

  python3 verify/run.sh verify/samples/sample.html "1440,768,375"
  bash    verify/run.sh verify/samples/sample.html "1440,768,375"

(Naming the interpreter is mandatory: the workspace sits on a FUSE mount where
the executable bit cannot be set, so `./verify/run.sh` fails with exit 126.)

It builds nothing (the HTML must already exist), runs browser_verify.py, prints
the JSON report to stdout, and exits non-zero when the page is dirty:

  * harness reported ok == false, or
  * any console error / page error occurred, or
  * any viewport showed horizontal document overflow, or
  * any <section> reported contained == false.

All problems are listed on stderr; the JSON on stdout is always complete.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile


def _fail(msg, code=2):
    sys.stderr.write("verify: FAIL: %s\n" % msg)
    return code


def main(argv=None):
    ap = argparse.ArgumentParser(
        prog="run.sh",
        description="Run the headless-browser geometry harness on a local HTML file.",
    )
    ap.add_argument("html", help="path to the HTML file to verify")
    ap.add_argument(
        "viewports",
        nargs="?",
        default="1440,768,375",
        help='comma-separated viewport widths, e.g. "1440,768,375" (default: 1440,768,375)',
    )
    ap.add_argument("--fonts-dir", action="append", default=[],
                    help="directory of local fonts to register via @font-face (repeatable)")
    ap.add_argument("--out-json", default=None, help="also write the raw JSON report here")
    # Anything we do not recognise is forwarded verbatim to browser_verify.py
    # (e.g. --tolerance 2). parse_known_args keeps options usable *after* the
    # positional arguments, which argparse.REMAINDER would have swallowed.
    args, extra = ap.parse_known_args(argv)

    here = os.path.dirname(os.path.abspath(__file__))
    script = os.path.join(here, "browser_verify.py")
    if not os.path.isfile(script):
        return _fail("missing harness script: %s" % script)
    if not os.path.isfile(args.html):
        return _fail("HTML file not found: %s" % args.html)

    tmpdir = tempfile.mkdtemp(prefix="ple-verify-")
    jpath = os.path.join(tmpdir, "report.json")
    try:
        cmd = [sys.executable, script, "--html", os.path.abspath(args.html),
               "--viewport", args.viewports, "--json", jpath]
        for d in args.fonts_dir:
            cmd += ["--fonts-dir", d]
        cmd += [a for a in extra if a != "--"]

        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=900)
        except subprocess.TimeoutExpired:
            return _fail("harness timed out after 900s")

        if proc.stderr:
            sys.stderr.write(proc.stderr)
        if not os.path.isfile(jpath):
            sys.stderr.write(proc.stdout)
            return _fail("harness produced no report (exit %d)" % proc.returncode)

        with open(jpath, "r", encoding="utf-8") as fh:
            data = json.load(fh)

        if args.out_json:
            shutil.copyfile(jpath, args.out_json)

        sys.stdout.write(json.dumps(data, indent=2, ensure_ascii=False) + "\n")

        problems = []
        if not data.get("ok"):
            problems.append("harness ok=false (%s)" % "; ".join(data.get("warnings") or ["no detail"]))
        for err in data.get("console_errors") or []:
            problems.append("console error: %s" % err)
        for vp in data.get("viewports") or []:
            if vp.get("error"):
                problems.append("width %s: %s" % (vp.get("width"), vp["error"]))
            if vp.get("horizontal_overflow"):
                problems.append("width %s: horizontal overflow of %spx" % (vp.get("width"), vp.get("overflow_px")))
            for sec in vp.get("sections") or []:
                if not sec.get("contained"):
                    problems.append(
                        "width %s: section #%s not contained (child escapes by %spx)"
                        % (vp.get("width"), sec.get("id"), sec.get("overflow_px"))
                    )
            for el in vp.get("overflowing_elements") or []:
                # Sections are `overflow: hidden`, so content that outgrows its own box is clipped
                # rather than reported as page overflow. Catch it here.
                problems.append(
                    "width %s: <%s> overflows its own box (%s > %s) around %r"
                    % (vp.get("width"), el.get("tag"), el.get("scrollWidth"), el.get("clientWidth"),
                       (el.get("text") or "")[:60])
                )
        if problems:
            for p in problems:
                sys.stderr.write("verify: FAIL: %s\n" % p)
            sys.stderr.write("verify: FAIL: %d problem(s) across %d viewport(s)\n"
                             % (len(problems), len(data.get("viewports") or [])))
            return 1

        n_texts = sum(len(vp.get("texts") or []) for vp in data.get("viewports") or [])
        n_secs = sum(len(vp.get("sections") or []) for vp in data.get("viewports") or [])
        sys.stderr.write("verify: PASS -- %d viewport(s), %d section(s), %d text node(s), no problems\n"
                         % (len(data.get("viewports") or []), n_secs, n_texts))
        return 0
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
