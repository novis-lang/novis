#!/usr/bin/env python3
"""Run Novis snippets against PHP, several at a time, in the shape a `.nvst` case already has.

    python tools/try.py .agent-tmp/promo.nvst .agent-tmp/div.nvst .agent-tmp/shift.nvst
    python tools/try.py .agent-tmp/*.nvst              # the whole scratch pad
    python tools/try.py --keep .agent-tmp/promo.nvst   # leave the generated .nvs/.php behind
    python tools/try.py --bundle examples/hello.nvs --expect "Hello, World!"

Each argument is a file in the `.nvst` shape -- `--TEST--`, `--FILE--`, and optionally
`--ORACLE--` -- or, when it holds no markers at all, a bare `<?nvs` snippet. For each one this
runs the Novis binary; where there is an `--ORACLE--` it runs PHP over that too and says whether the
two agree, with the first line they differ on.

## Why this exists

Measured over a 33-session run, sessions made **370 calls that ran a snippet, 335 of them
distinct** -- 9.6 a session. Every one was the same shape: heredoc a scratch `.nvs` into
`.agent-tmp`, run it, then hand-write a `php -r` beside it to see what PHP does. Two costs sit in
that, and this addresses both.

The first is turns. A turn's time-to-first-token is ~80% of its clock and does not depend on what
the turn does, so nine experiments run one at a time cost nine round trips for work that is
embarrassingly parallel. Here they are one call -- and inside it they run several at a time, as
wide as `tools/machine.py` says this box may go, printed back in the order they were asked for.

The second is correctness, and it matters more. **Priority 2 is PHP-compatible observable
behaviour**, and a hand-written `php -r` twin is a *translation* -- performed under time pressure,
by the same agent that wrote the Novis, at the moment it most wants the answer to be yes. A twin
that quietly differs from the Novis it is checking proves nothing and reads exactly like proof. Here
the twin is a section of the same file, run by the same tool that will run it in
`tests/differential/`, printed side by side.

That is the other half of the shape: an experiment that comes out right **is already the case**.
Give the file a `--TEST--` line, move it under `tests/differential/`, and `nvs test` runs the same
two programs the same way -- no second translation step, which is where the twin used to drift.
`conventions.md` § *A `.nvst` test case* is the format's home; this file never restates it.

Nothing here judges. A snippet that fails to compile prints its diagnostic, a twin that diverges
prints both outputs, and the exit status is 0 unless a snippet could not be *read* -- because
"Novis and PHP disagree" is the finding, not an error.

## `--bundle`: the other twin

`--bundle FILE...` swaps the twin. Each argument is a `.nvs` entry point rather than a `.nvst`
snippet; each is built with `nvs build --compile`, and the executable that comes out is run beside
`nvs run` over the same source. That pair *is*
`rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s own verification list -- "a
bundled executable runs identically to `nvs run` against the same source" -- asked the same way a
`--ORACLE--` asks PHP, and for the same reason: a hand-written expectation about a bundle is a
translation, and the executable is right there to ask instead.

`--expect LINE` is the one place this file judges, and it exists because
`docs/agent/loop-goal.toml`'s acceptance check needs a verdict rather than a report: every `LINE`  # check-links:retired
given must appear in the bundle's own output, and the exit status is 1 when one does not.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import machine  # noqa: E402  -- same directory; how wide anything runs has one home and it is there

ROOT = Path(__file__).resolve().parent.parent
TMP = ROOT / ".agent-tmp"
BINARY = ROOT / "target" / "debug" / ("nvs.exe" if os.name == "nt" else "nvs")

#: A `--SECTION--` line in a `.nvst` file. `crates/nvs-test/src/case.rs` is the authority on the
#: full roster; only the three below mean anything here, and an unknown one is ignored rather
#: than refused -- this is a scratch pad, not the case runner.
SECTION_RE = re.compile(r"^--([A-Z][A-Z0-9-]*)--\s*$", re.M)

#: How long one snippet may run. A hung experiment is a bug in the snippet, and reporting it as a
#: timeout beside the others is more useful than blocking the whole call on it.
TIMEOUT = 30


def sections(text: str) -> dict[str, str]:
    """`{SECTION: body}` for a `.nvst` file, or `{"FILE": text}` for a bare snippet."""
    marks = list(SECTION_RE.finditer(text))
    if not marks:
        return {"FILE": text}
    found = {}
    for i, m in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(text)
        found[m.group(1)] = text[m.end():end]
    return found


def run(cmd: list[str], cwd: Path) -> tuple[str, int]:
    try:
        proc = subprocess.run(
            # UTF-8 explicitly, because `text=True` decodes with the console's
            # own code page and Windows hands back cp1252 — under which a case
            # echoing one of `rule:tooling/terminal-output-is-a-sink`'s Control Pictures (`␛` is
            # `E2 90 9B`) raises `UnicodeDecodeError` in the reader thread
            # rather than failing the comparison. `replace` for the same
            # reason: a program under test may write bytes that are not UTF-8
            # at all, and that is a difference to show, not a crash.
            cmd, cwd=cwd, capture_output=True, text=True, timeout=TIMEOUT,
            encoding="utf-8", errors="replace",
        )
    except FileNotFoundError:
        return (f"<{cmd[0]} not found on PATH>", 127)
    except subprocess.TimeoutExpired:
        return (f"<no answer in {TIMEOUT}s>", 124)
    return (proc.stdout + proc.stderr, proc.returncode)


def first_difference(a: str, b: str) -> str:
    """The first line the two outputs disagree on, as `nvs` / `php` beside each other."""
    left, right = a.split("\n"), b.split("\n")
    for n, (x, y) in enumerate(zip(left, right), start=1):
        if x != y:
            return (f"    first difference at line {n}\n"
                    f"      nvs: {x[:90]!r}\n"
                    f"      php: {y[:90]!r}")
    if len(left) != len(right):
        longer, who = (left, "nvs") if len(left) > len(right) else (right, "php")
        at = min(len(left), len(right))
        return (f"    both agree for {at} line(s); {who} then has "
                f"{abs(len(left) - len(right))} more, starting {longer[at][:90]!r}")
    return "    the outputs differ in trailing whitespace only"


def one(path: Path, keep: bool, php: str, stem: str) -> tuple[bool, list[str]]:
    """Run one snippet. True when Novis and its twin agree, or when there is no twin to disagree.

    Returns its output instead of printing it: several snippets run at once, and interleaved
    blocks would be unreadable. `main` prints them back in the order they were asked for, so the
    concurrency is invisible in the answer -- which is the only property it must have.

    `stem` names this snippet's generated files. It is not `path.stem`, because two arguments may
    share one and two workers writing `.agent-tmp/try-x.nvs` at the same time is a race.
    """
    out: list[str] = []
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        return True, [f"===== {path}  -- cannot read: {exc}"]

    parts = sections(text)
    title = parts.get("TEST", "").strip().split("\n")[0]
    out.append(f"===== {path.name}" + (f"  -- {title[:90]}" if title else ""))

    body = parts.get("FILE")
    if body is None or not body.strip():
        out.append("  no `--FILE--` section and no bare snippet -- nothing to run")
        return True, out

    TMP.mkdir(parents=True, exist_ok=True)
    nvs_file = TMP / f"try-{stem}.nvs"
    nvs_file.write_text(body.lstrip("\n"), encoding="utf-8", newline="")
    nvs_out, nvs_code = run([str(BINARY), "run", str(nvs_file)], ROOT)

    out.append(f"  nvs  exit {nvs_code}")
    out.extend(f"    | {line}" for line in nvs_out.rstrip("\n").split("\n"))

    diverges = parts.get("ORACLE-DIVERGES")
    if diverges is not None:
        out.append(f"  oracle: deliberately diverges -- {diverges.strip().split(chr(10))[0][:90]}")
        return True, out

    twin = parts.get("ORACLE")
    if twin is None:
        out.append("  no `--ORACLE--`: this ran Novis only. A twin here is what makes the answer")
        out.append("  evidence rather than an opinion -- and makes the file a differential case.")
        return True, out

    php_file = TMP / f"try-{stem}.php"
    php_file.write_text(twin.lstrip("\n"), encoding="utf-8", newline="")
    php_out, php_code = run([php, str(php_file)], ROOT)

    out.append(f"  php  exit {php_code}")
    out.extend(f"    | {line}" for line in php_out.rstrip("\n").split("\n"))

    if not keep:
        for f in (nvs_file, php_file):
            try:
                f.unlink()
            except OSError:
                pass

    agree = nvs_out == php_out
    out.append("  MATCH" if agree else "  DIFFER")
    if not agree:
        out.append(first_difference(nvs_out, php_out))
    return agree, out


def bundled(path: Path, keep: bool, expect: list[str], stem: str) -> tuple[bool, list[str]]:
    """Build `path` with `nvs build --compile`, run it, and hold it against `nvs run`.

    True when the bundle built, ran, agreed with `nvs run` line for line, and printed every
    `--expect` line. Returns its output for the same reason `one` does: several of these run at
    once and interleaved blocks would be unreadable.
    """
    out: list[str] = [f"===== {path.name}  -- bundled"]
    if not path.exists():
        return False, out + [f"  cannot read {path}"]

    TMP.mkdir(parents=True, exist_ok=True)
    exe = TMP / (f"bundle-{stem}.exe" if os.name == "nt" else f"bundle-{stem}")
    build_out, build_code = run(
        [str(BINARY), "build", "--compile", str(path), "-o", str(exe)], ROOT
    )
    out.append(f"  build  exit {build_code}")
    out.extend(f"    | {line}" for line in build_out.rstrip("\n").split("\n"))
    if build_code != 0 or not exe.exists():
        return False, out

    bundle_out, bundle_code = run([str(exe)], ROOT)
    out.append(f"  bundle exit {bundle_code}")
    out.extend(f"    | {line}" for line in bundle_out.rstrip("\n").split("\n"))

    run_out, run_code = run([str(BINARY), "run", str(path)], ROOT)
    out.append(f"  nvs run exit {run_code}")

    if not keep:
        try:
            exe.unlink()
        except OSError:
            pass

    ok = True
    if bundle_out == run_out and bundle_code == run_code:
        out.append("  MATCH -- the bundle runs identically to `nvs run` (`rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host` Verification)")
    else:
        ok = False
        out.append("  DIFFER -- the bundle and `nvs run` do not agree")
        out.append(
            first_difference(bundle_out, run_out)
            .replace("nvs:", "bundle:")
            .replace("php:", "nvs run:")
        )
    for line in expect:
        if line not in bundle_out:
            ok = False
            out.append(f"  MISSING -- the bundle never printed {line!r}")
    return ok, out


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("files", nargs="+", metavar="FILE",
                    help="`.nvst`-shaped snippets, or bare `<?nvs` ones; as many as you have questions")
    ap.add_argument("--keep", action="store_true",
                    help="leave the generated .nvs/.php under .agent-tmp/ instead of removing them")
    ap.add_argument("--php", default="php", help="the PHP executable (default `php`)")
    ap.add_argument("--bundle", action="store_true",
                    help="treat each FILE as a `.nvs` entry point: build it with "
                         "`nvs build --compile` and run the result beside `nvs run`")
    ap.add_argument("--expect", action="append", default=[], metavar="LINE",
                    help="with --bundle: a line the bundle's output must contain; "
                         "a miss is a non-zero exit. Repeatable")
    opts = ap.parse_args()
    if opts.expect and not opts.bundle:
        ap.error("--expect is only meaningful with --bundle")

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    if not BINARY.exists():
        print(f"try.py: {BINARY.relative_to(ROOT)} is not built.")
        print("        `cargo build` first -- and note that `nv verify` builds it too,")
        print("        so a snippet run right after a green verification needs nothing.")
        return 2

    paths = [Path(name) for name in opts.files]
    # One `.agent-tmp` name per snippet, unique even when two arguments share a stem.
    stems, seen = [], set()
    for i, p in enumerate(paths):
        stems.append(p.stem if p.stem not in seen else f"{p.stem}-{i}")
        seen.add(stems[-1])

    # A snippet is a process pair with nothing shared, so the width is the machine's to decide and
    # `tools/machine.py` decides it -- half the cores, at most one worker per snippet. This is the
    # second half of "in one call": the call stopped being nine round trips when this file was
    # written, and stops being nine sequential runs here.
    jobs = machine.jobs("local", ceiling=len(paths), envs=("NVS_TRY_JOBS",))

    if opts.bundle:
        work = lambda a: bundled(a[0], opts.keep, opts.expect, a[1])  # noqa: E731
    else:
        work = lambda a: one(a[0], opts.keep, opts.php, a[1])  # noqa: E731

    agreed = 0
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        # `map` keeps input order: the blocks print in the order they were asked for, whatever
        # order they finished in.
        for i, (agree, block) in enumerate(pool.map(work, zip(paths, stems))):
            if i:
                print()
            print("\n".join(block))
            agreed += bool(agree)

    print()
    twins = len(opts.files) - agreed
    noun = "entry point" if opts.bundle else "snippet"
    print(f"-- try: {len(opts.files)} {noun}(s) in one call"
          + (f", {jobs} at a time" if jobs > 1 else "")
          + (f", {twins} disagreeing with its twin" if twins else ""))
    # `--bundle` is the one mode that judges: the acceptance check that drives it needs a verdict.
    return 1 if opts.bundle and twins else 0


if __name__ == "__main__":
    sys.exit(main())
