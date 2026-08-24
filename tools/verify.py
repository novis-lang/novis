#!/usr/bin/env python3
"""AGENTS.md § *Session workflow* step 3, as one command.

`cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
in that order, stopping at the first failure. Green prints one line per step; a failure prints
that step's output and nothing else.

The point is turn count, not typing. Run separately, those four are four tool calls whose
combined output runs to tens of thousands of tokens a session never reads once it is green --
and a session's wall clock is very nearly its number of turns times a constant. Run here, a
green verification is one call and about ten lines.

    python tools/verify.py                  # the four steps
    python tools/verify.py -p mwl-ir        # scope build/test/clippy to one package
    python tools/verify.py --fast           # build and test only, for a mid-work check
    python tools/verify.py --full           # do not truncate the failing step's output

Full output of every step is always written to `.agent-tmp/verify-<step>.log`, whether it
passed or not, so a truncated failure is one Read away from complete.

This script judges nothing. A step's own exit status is the whole verdict -- there is no
threshold here, no allowance, and no way to make a red step green from this file.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TMP = ROOT / ".agent-tmp"

TAIL_LINES = 60  # of the failing step only; the full log is always on disk

# `cargo test` prints one of these per test binary.
RESULT_RE = re.compile(r"test result: \w+\. (\d+) passed; (\d+) failed")
# clippy/rustc summary lines worth surfacing above the raw tail.
WARN_RE = re.compile(r"^(warning|error)(\[[^\]]+\])?: (.*)$", re.MULTILINE)


class Step:
    def __init__(self, name, args, summarize):
        self.name = name
        self.args = args
        self.summarize = summarize
        self.seconds = 0.0
        self.code = None
        self.out = ""

    @property
    def cmd(self):
        return "cargo " + " ".join(self.args)


def run(step):
    started = time.monotonic()
    try:
        p = subprocess.run(
            ["cargo", *step.args],
            cwd=ROOT,
            capture_output=True,
            encoding="utf-8",
            errors="replace",
        )
        step.code, step.out = p.returncode, (p.stdout or "") + (p.stderr or "")
    except OSError as exc:
        step.code, step.out = -1, f"could not run `{step.cmd}`: {exc}"
    step.seconds = time.monotonic() - started

    TMP.mkdir(exist_ok=True)
    (TMP / f"verify-{step.name}.log").write_text(step.out, encoding="utf-8", newline="\n")
    return step.code == 0


# ------------------------------------------------------------------ summaries


def summarize_build(out):
    return "ok"


def summarize_test(out):
    suites = RESULT_RE.findall(out)
    passed = sum(int(a) for a, _ in suites)
    failed = sum(int(b) for _, b in suites)
    if not suites:
        return "ran, but printed no `test result:` line -- check the log"
    return f"{passed} passed, {failed} failed  ({len(suites)} suites)"


def summarize_clippy(out):
    n = len([m for m in WARN_RE.finditer(out) if m.group(1) == "warning"])
    return "no warnings" if n == 0 else f"{n} warning(s)"


def summarize_fmt(out):
    return "clean"


def steps_for(opts):
    scope = ["-p", opts.package] if opts.package else []
    steps = [
        Step("build", ["build", *scope], summarize_build),
        Step("test", ["test", *scope], summarize_test),
    ]
    if not opts.fast:
        steps.append(
            Step("clippy", ["clippy", "--all-targets", *scope, "--", "-D", "warnings"],
                 summarize_clippy)
        )
        # `cargo fmt --check` takes no -p in the shape this workspace uses it.
        steps.append(Step("fmt", ["fmt", "--check"], summarize_fmt))
    return steps


def clock(seconds):
    if seconds >= 60:
        return f"{int(seconds) // 60}m{int(seconds) % 60:02d}s"
    return f"{seconds:.0f}s"


def tail(text, limit):
    lines = text.rstrip("\n").split("\n")
    if limit <= 0 or len(lines) <= limit:
        return "\n".join(lines), 0
    return "\n".join(lines[-limit:]), len(lines) - limit


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("-p", "--package", help="scope build/test/clippy to one package")
    ap.add_argument("--fast", action="store_true", help="build and test only")
    ap.add_argument("--full", action="store_true", help="do not truncate the failing step")
    opts = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    steps = steps_for(opts)
    done = []
    failed = None
    for step in steps:
        if not run(step):
            failed = step
            break
        done.append(step)

    total = sum(s.seconds for s in done) + (failed.seconds if failed else 0)
    scope = f" (-p {opts.package})" if opts.package else ""

    if failed is None:
        print(f"verify: {len(done)} of {len(steps)} green in {clock(total)}{scope}")
        for s in done:
            print(f"  {s.name:<8} {clock(s.seconds):>6}   {s.summarize(s.out)}")
        return 0

    print(f"verify: FAILED at {failed.name} "
          f"(step {len(done) + 1} of {len(steps)}) after {clock(total)}{scope}")
    for s in done:
        print(f"  {s.name:<8} {clock(s.seconds):>6}   {s.summarize(s.out)}")
    print(f"  {failed.name:<8} {'---':>6}   exit {failed.code}")

    body, hidden = tail(failed.out, 0 if opts.full else TAIL_LINES)
    print(f"\n-- `{failed.cmd}`" + (f", last {TAIL_LINES} lines" if hidden else ""))
    print(body)
    if hidden:
        print(f"\n... {hidden} earlier line(s) hidden")
    print(f"\nfull output: .agent-tmp/verify-{failed.name}.log")
    return 1


if __name__ == "__main__":
    sys.exit(main())
