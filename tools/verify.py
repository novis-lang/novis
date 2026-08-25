#!/usr/bin/env python3
"""AGENTS.md § *Session workflow* step 3, as one command.

`cargo build`, `cargo fmt --check`, `cargo test` and `cargo clippy --all-targets -- -D warnings`
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
    python tools/verify.py --no-cache       # re-run even if the tree is provably unchanged

Full output of every step is always written to `.agent-tmp/verify-<step>.log`, whether it
passed or not, so a truncated failure is one Read away from complete.

This script judges nothing. A step's own exit status is the whole verdict -- there is no
threshold here, no allowance, and no way to make a red step green from this file.

## Why `fmt` runs second

It costs a second and needs no build, so a formatting slip should not cost a whole run --
`.loop/logs` holds runs that paid `build=4s test=41s clippy=5s` before failing at `fmt`, and
sessions had started typing `cargo fmt --all && python tools/verify.py` to work around it. It
runs *second* rather than first because `cargo fmt --check` on unparseable code reports a
rustfmt parse error, which is a far worse diagnostic than the one `cargo build` would have
given for the same typo.

## Why a second run on an unchanged tree is free

The rule is one verification per session, at the end. Measured against `.loop/logs`, 33 of 41
sessions ran this script more than once -- 2.4 times on average -- and most of those re-runs
came after step 4 edited only documentation. Prose cannot break a build, so those runs paid
about forty seconds to re-derive a verdict they already held.

So the verdict is cached against a content hash of everything `cargo` reads: every file under
`crates/`, `benches/`, `tests/` and `examples/`, the workspace manifests, `rustfmt.toml`,
`rust-toolchain.toml` and the exact `rustc -vV`. A repeat run whose hash matches prints the
cached verdict and exits, in about a fifth of a second. This is **not** a check being skipped:
the inputs are bit-identical, so re-running the same compiler over them cannot reach a
different answer. Only a *green* verdict is cached, the entry expires after an hour, and
`--no-cache` forces the real thing.

The cache records the scope and the step list it was produced by, so a `--fast` verdict never
satisfies a full run and a `-p mwl-ir` verdict never satisfies an unscoped one; the reverse
directions do, because a superset already proved the subset. Anything unexpected -- an
unreadable file, a corrupt cache -- makes it fall through and run the steps for real.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TMP = ROOT / ".agent-tmp"
CACHE = TMP / "verify-green.json"

TAIL_LINES = 60  # of the failing step only; the full log is always on disk
CACHE_TTL = 3600  # seconds. A tree hash cannot go stale on its own; this is a belt on braces.

# Everything cargo reads, relative to ROOT. Directories are walked in full -- a `.mwlt`
# fixture, an insta `.snap` and a `Cargo.toml` all change what the steps will answer.
INPUT_DIRS = ("crates", "benches", "tests", "examples")
INPUT_FILES = ("Cargo.toml", "Cargo.lock", "rustfmt.toml", "rust-toolchain.toml")

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
    steps = [Step("build", ["build", *scope], summarize_build)]
    if not opts.fast:
        # `cargo fmt --check` takes no -p in the shape this workspace uses it, and it is a
        # second, so it goes ahead of the two expensive steps -- see the module docstring.
        steps.append(Step("fmt", ["fmt", "--check"], summarize_fmt))
    steps.append(Step("test", ["test", *scope], summarize_test))
    if not opts.fast:
        steps.append(
            Step("clippy", ["clippy", "--all-targets", *scope, "--", "-D", "warnings"],
                 summarize_clippy)
        )
    return steps


# ------------------------------------------------------------------ the green cache


def rustc_version():
    p = subprocess.run(["rustc", "-vV"], capture_output=True, encoding="utf-8", errors="replace")
    return (p.stdout or "") + (p.stderr or "")


def input_paths():
    """Every file the four steps read, sorted, as ROOT-relative posix strings."""
    seen = []
    for name in INPUT_FILES:
        if (ROOT / name).is_file():
            seen.append(name)
    for top in INPUT_DIRS:
        base = ROOT / top
        if not base.is_dir():
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            # A nested `target/` is cargo's own output, never an input.
            dirnames[:] = [d for d in dirnames if d != "target"]
            rel = Path(dirpath).relative_to(ROOT)
            seen.extend((rel / f).as_posix() for f in filenames)
    seen.sort()
    return seen


def tree_key():
    """A content hash of every input, or None if anything at all goes wrong."""
    try:
        h = hashlib.blake2b(digest_size=16)
        h.update(rustc_version().encode("utf-8", "replace"))
        for rel in input_paths():
            h.update(rel.encode("utf-8"))
            h.update(b"\0")
            h.update((ROOT / rel).read_bytes())
            h.update(b"\0")
        return h.hexdigest()
    except OSError:
        return None


def cached_verdict(key, opts, steps):
    """The stored verdict if it provably covers this request, else None."""
    if key is None or opts.no_cache:
        return None
    try:
        entry = json.loads(CACHE.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    if entry.get("key") != key:
        return None
    if time.time() - float(entry.get("when", 0)) > CACHE_TTL:
        return None
    # A scoped verdict cannot stand in for an unscoped one; the reverse is fine.
    if entry.get("package") not in (None, opts.package):
        return None
    if not set(s.name for s in steps) <= set(entry.get("steps", {})):
        return None
    return entry


def store_verdict(key, opts, steps):
    if key is None:
        return
    try:
        TMP.mkdir(exist_ok=True)
        CACHE.write_text(
            json.dumps(
                {
                    "key": key,
                    "when": time.time(),
                    "package": opts.package,
                    "steps": {s.name: s.summarize(s.out) for s in steps},
                    "seconds": round(sum(s.seconds for s in steps), 1),
                },
                indent=1,
            ),
            encoding="utf-8",
            newline="\n",
        )
    except OSError:
        pass


def drop_verdict():
    try:
        CACHE.unlink(missing_ok=True)
    except OSError:
        pass


# ------------------------------------------------------------------ output


def clock(seconds):
    if seconds >= 60:
        return f"{int(seconds) // 60}m{int(seconds) % 60:02d}s"
    return f"{seconds:.0f}s"


def ago(seconds):
    if seconds < 90:
        return f"{int(seconds)}s ago"
    return f"{int(seconds) // 60}m ago"


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
    ap.add_argument("--no-cache", action="store_true",
                    help="re-run the steps even if the tree is provably unchanged")
    opts = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    steps = steps_for(opts)
    scope = f" (-p {opts.package})" if opts.package else ""

    key = tree_key()
    hit = cached_verdict(key, opts, steps)
    if hit is not None:
        print(f"verify: green, tree unchanged since {ago(time.time() - hit['when'])}"
              f" -- nothing to re-run{scope}")
        for name, summary in hit["steps"].items():
            print(f"  {name:<8} {'--':>6}   {summary}")
        print(f"\nthat verdict cost {clock(hit['seconds'])} and covers this tree exactly; "
              f"`--no-cache` runs it again anyway.")
        return 0

    done = []
    failed = None
    for step in steps:
        if not run(step):
            failed = step
            break
        done.append(step)

    total = sum(s.seconds for s in done) + (failed.seconds if failed else 0)

    if failed is None:
        store_verdict(key, opts, steps)
        print(f"verify: {len(done)} of {len(steps)} green in {clock(total)}{scope}")
        for s in done:
            print(f"  {s.name:<8} {clock(s.seconds):>6}   {s.summarize(s.out)}")
        return 0

    drop_verdict()
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
