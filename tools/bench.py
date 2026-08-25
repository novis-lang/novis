#!/usr/bin/env python3
"""Run the userland benchmark suite: the same program, once in MWL and once in PHP.

    python tools/bench.py                      # every case, 5 timed reps each
    python tools/bench.py 05 regex             # only cases whose name contains "05" or "regex"
    python tools/bench.py --reps 9             # more reps when a number looks noisy
    python tools/bench.py --check              # correctness only: do the two agree? (no timing)
    python tools/bench.py --php-mode default   # PHP as installed, instead of with opcache+JIT
    python tools/bench.py --json docs/perf/userland.ndjson   # append one record per case

The cases live in `benches/userland/` as pairs -- `NN-slug.mwl` and `NN-slug.php` -- and
[its README](../benches/userland/README.md) owns what a case is and how to add one. This file
owns only how they are *measured*.

## What is measured, and what the number means

Wall-clock of the whole process, `min` over N reps after one discarded warm-up. Minimum rather
than mean because the thing being estimated is the cost of the work, and every source of noise
on a desktop -- a scheduler slice, a page fault, an antivirus scan -- adds time and never
subtracts it. The median is printed beside it so a large min/median gap says "this machine was
busy" out loud rather than hiding inside an average.

`00-baseline` is a case like any other, but its time is also **subtracted** from every other
case in the `work` columns: both engines pay a fixed cost to start a process, read a file and
produce code before a single line of the benchmark runs, and over a 30 ms case that cost is the
measurement. Read `total` for "what does this script cost me at the command line" and `work` for
"how fast is the language". They answer different questions and the suite refuses to pick one.

The `x` column is `php / mwl` on the `work` figures: **above 1.0 means MWL is faster**, and 0.5
means MWL takes twice as long. It is a ratio of two numbers measured on the same machine within
seconds of each other, which is the only comparison a wall-clock figure supports --
[ADR 0026](../docs/adr/0026-performance-measurement-methodology.md) is why a cross-machine
history is counted in instructions instead, and this suite is that ADR's § 3 secondary figure
rather than a competitor to it.

## Correctness is a gate on the timing, not a separate run

Every case prints a result, and the two engines' stdout must match byte for byte before either
time is reported. A benchmark that has quietly stopped doing the same work in both languages is
worse than no benchmark, and the cheapest moment to catch it is the run that would otherwise
publish its number. A mismatch prints both outputs and that case reports `DIFF` with no timing.

## Release against release, always

A debug MWL build is one to two orders of magnitude slower than release and would say nothing
about the language, so a binary under `target/debug/` is refused by name unless `--allow-debug`
says the caller means it. Nothing here builds anything: the binary on disk is the binary that
runs, and a warning says so if it is older than the newest file under `crates/`.

PHP is run with opcache and the tracing JIT on (`--php-mode jit`, the default) because the
project's own target is stated against PHP *with* JIT, and a comparison against an engine that
has been asked to run slower proves nothing. `--php-mode default` runs it exactly as installed
-- the CLI's real out-of-the-box behaviour -- and the mode is recorded in every JSON record, so
the two are never silently mixed in one history file.

## Adding a measure later

`measure()` returns a dict, `COLUMNS` says which keys are printed and how, and everything else
threads dicts around without knowing what is in them. A second measure -- peak RSS, an
allocation count, an instruction count under callgrind -- is a key added there and a column
added here, not a change to the runner.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import statistics
import subprocess
import sys
import time
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):  # a case description is prose, and consoles here are cp1252
    sys.stdout.reconfigure(encoding="utf-8")

ROOT = Path(__file__).resolve().parent.parent
CASE_DIR = ROOT / "benches" / "userland"
BASELINE = "00-baseline"

# PHP invocation modes. The mode name is recorded in every JSON record so a history file cannot
# silently mix an engine that was allowed to JIT with one that was not.
PHP_MODES = {
    "jit": [
        "-d", "opcache.enable_cli=1",
        "-d", "opcache.jit_buffer_size=64M",
        "-d", "opcache.jit=tracing",
    ],
    "default": [],
}


class Case:
    """One benchmark: the same program written twice, and what it is for."""

    def __init__(self, name: str, mwl: Path, php: Path, description: str) -> None:
        self.name = name
        self.mwl = mwl
        self.php = php
        self.description = description

    @property
    def is_baseline(self) -> bool:
        return self.name == BASELINE


def discover(patterns: list[str]) -> list[Case]:
    """Every `NN-slug.mwl` with a `NN-slug.php` beside it, in name order."""
    cases = []
    for mwl in sorted(CASE_DIR.glob("*.mwl")):
        php = mwl.with_suffix(".php")
        if not php.exists():
            print(f"warning: {mwl.name} has no .php twin -- skipped", file=sys.stderr)
            continue
        cases.append(Case(mwl.stem, mwl, php, describe(mwl)))
    if patterns:
        kept = [c for c in cases if c.is_baseline or any(p in c.name for p in patterns)]
        if not any(not c.is_baseline for c in kept):
            sys.exit(f"no case matches {patterns!r}; `ls benches/userland` lists them")
        return kept
    return cases


def describe(path: Path) -> str:
    """A case describes itself in its first `//` line -- there is no second manifest to update."""
    for line in path.read_text(encoding="utf-8").splitlines()[:6]:
        line = line.strip()
        if line.startswith("//"):
            return line[2:].strip()
    return ""


def find_mwl(explicit: str | None, allow_debug: bool) -> Path:
    if explicit:
        binary = Path(explicit)
    else:
        exe = "mwl.exe" if os.name == "nt" else "mwl"
        binary = ROOT / "target" / "release" / exe
    if not binary.exists():
        sys.exit(
            f"no MWL binary at {binary}\n"
            "  build one with `cargo build --release`, or point --mwl at the one you mean"
        )
    if "debug" in binary.parts and not allow_debug:
        sys.exit(
            f"{binary} is a debug build, and a debug build measures the assertions rather than\n"
            "  the language. Use target/release, or --allow-debug if you truly mean this one."
        )
    return binary


def warn_if_stale(binary: Path) -> None:
    """Nothing here builds. If the binary predates the sources, the number is about old code."""
    try:
        built = binary.stat().st_mtime
        newest = max(
            p.stat().st_mtime for p in (ROOT / "crates").rglob("*.rs") if p.is_file()
        )
    except (OSError, ValueError):
        return
    if newest > built:
        age = (newest - built) / 3600
        print(
            f"warning: {binary.name} is {age:.1f} h older than the newest file under crates/ --"
            " these numbers are about the build on disk, not the tree",
            file=sys.stderr,
        )


def run_once(argv: list[str]) -> tuple[float, str, str, int]:
    """One process, timed end to end. Wall-clock includes start-up on purpose -- see the header."""
    start = time.perf_counter()
    proc = subprocess.run(argv, capture_output=True, text=True, cwd=ROOT)
    elapsed = time.perf_counter() - start
    return elapsed, proc.stdout, proc.stderr, proc.returncode


def measure(argv: list[str], reps: int) -> dict:
    """The measurement seam: one dict per engine per case.

    A future measure -- peak RSS, allocation count, instruction count -- is another key set
    here and another entry in COLUMNS. Nothing between the two needs to know it exists.
    """
    _, stdout, stderr, code = run_once(argv)  # warm-up, discarded
    if code != 0:
        return {"failed": True, "stdout": stdout, "stderr": stderr, "code": code}
    samples = []
    for _ in range(reps):
        elapsed, out, err, code = run_once(argv)
        if code != 0 or out != stdout:
            return {
                "failed": True,
                "stdout": out,
                "stderr": err or "output was not stable across reps",
                "code": code,
            }
        samples.append(elapsed * 1000.0)
    return {
        "failed": False,
        "stdout": stdout,
        "stderr": stderr,
        "min_ms": min(samples),
        "median_ms": statistics.median(samples),
        "samples_ms": samples,
    }


# name, header, width, how to read one row's value out of the joined record
COLUMNS = [
    ("mwl", "mwl ms", 9, lambda r: fmt(r["mwl"].get("min_ms"))),
    ("php", "php ms", 9, lambda r: fmt(r["php"].get("min_ms"))),
    ("mwl_work", "mwl work", 9, lambda r: fmt(r.get("mwl_work_ms"))),
    ("php_work", "php work", 9, lambda r: fmt(r.get("php_work_ms"))),
    ("ratio", "php/mwl", 9, lambda r: ratio(r.get("ratio"))),
]


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:,.1f}"


def ratio(value: float | None) -> str:
    if value is None:
        return "-"
    return f"{value:.2f}x"


def evaluate(case: Case, mwl_argv: list[str], php_argv: list[str], reps: int) -> dict:
    mwl = measure(mwl_argv, reps)
    php = measure(php_argv, reps)
    record = {"case": case.name, "description": case.description, "mwl": mwl, "php": php}
    if mwl["failed"] or php["failed"]:
        record["status"] = "ERROR"
    elif mwl["stdout"] != php["stdout"]:
        record["status"] = "DIFF"
    else:
        record["status"] = "ok"
    return record


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run the MWL/PHP userland benchmark suite side by side.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__.split("## What is measured", 1)[0],
    )
    parser.add_argument("patterns", nargs="*", help="substrings; only matching cases run")
    parser.add_argument("--reps", type=int, default=5, help="timed reps per engine (default 5)")
    parser.add_argument("--check", action="store_true", help="agreement only, no timing")
    parser.add_argument("--mwl", help="path to the mwl binary (default target/release)")
    parser.add_argument("--php", default="php", help="php executable (default `php`)")
    parser.add_argument(
        "--php-mode",
        choices=sorted(PHP_MODES),
        default="jit",
        help="`jit` (default) runs PHP with opcache+tracing JIT; `default` runs it as installed",
    )
    parser.add_argument("--allow-debug", action="store_true", help="permit a debug MWL binary")
    parser.add_argument("--json", metavar="PATH", help="append one NDJSON record per case")
    args = parser.parse_args()

    if args.reps < 1:
        sys.exit("--reps must be at least 1")
    reps = 1 if args.check else args.reps

    binary = find_mwl(args.mwl, args.allow_debug)
    warn_if_stale(binary)
    php_flags = PHP_MODES[args.php_mode]
    cases = discover(args.patterns)

    print(f"mwl  {binary}  ({version(binary, ['--version'])})")
    print(f"php  {args.php} [{args.php_mode}]  ({version(args.php, ['-r', 'echo PHP_VERSION;'])})")
    print(f"{len(cases)} case(s), {reps} rep(s), min of reps; php/mwl above 1.00 means MWL is faster")
    print()

    records = []
    for case in cases:
        record = evaluate(
            case,
            [str(binary), "run", str(case.mwl)],
            [args.php, *php_flags, str(case.php)],
            reps,
        )
        records.append(record)
        print(f"  {record['status']:<5} {case.name}", flush=True)

    baseline = next((r for r in records if r["case"] == BASELINE and r["status"] == "ok"), None)
    for record in records:
        if record["status"] != "ok" or not baseline:
            continue
        record["mwl_work_ms"] = max(
            0.0, record["mwl"]["min_ms"] - baseline["mwl"]["min_ms"]
        )
        record["php_work_ms"] = max(
            0.0, record["php"]["min_ms"] - baseline["php"]["min_ms"]
        )
        if record["mwl_work_ms"] > 0:
            record["ratio"] = record["php_work_ms"] / record["mwl_work_ms"]

    print()
    if args.check:
        report_check(records)
    else:
        report_table(records)

    if args.json:
        write_json(Path(args.json), records, binary, args.php, args.php_mode, reps)

    return 0 if all(r["status"] == "ok" for r in records) else 1


def version(executable, argv: list[str]) -> str:
    try:
        out = subprocess.run(
            [str(executable), *argv], capture_output=True, text=True, timeout=30
        )
        return out.stdout.strip().splitlines()[0] if out.stdout.strip() else "unknown"
    except (OSError, subprocess.SubprocessError, IndexError):
        return "unknown"


def report_check(records: list[dict]) -> None:
    for record in records:
        if record["status"] == "ok":
            print(f"ok    {record['case']}  ->  {record['mwl']['stdout'].strip()!r}")
        else:
            explain(record)


def report_table(records: list[dict]) -> None:
    width = max(len(r["case"]) for r in records)
    header = "case".ljust(width) + "".join(h.rjust(w) for _, h, w, _ in COLUMNS)
    print(header)
    print("-" * len(header))
    for record in records:
        if record["status"] != "ok":
            print(record["case"].ljust(width) + record["status"].rjust(9))
            continue
        print(record["case"].ljust(width) + "".join(read(record).rjust(w) for _, _, w, read in COLUMNS))
    print("-" * len(header))
    ok = [r for r in records if r["status"] == "ok" and "ratio" in r]
    if ok:
        best = max(ok, key=lambda r: r["ratio"])
        worst = min(ok, key=lambda r: r["ratio"])
        print(f"fastest against PHP  {best['case']}  {best['ratio']:.2f}x")
        print(f"slowest against PHP  {worst['case']}  {worst['ratio']:.2f}x")
        print(f"median ratio         {statistics.median(r['ratio'] for r in ok):.2f}x")
    for record in records:
        if record["status"] != "ok":
            print()
            explain(record)
    noisy = [
        r for r in records
        if r["status"] == "ok" and r["mwl"]["median_ms"] > 1.25 * r["mwl"]["min_ms"]
    ]
    if noisy:
        print(
            "note: min and median differ by more than 25% on "
            + ", ".join(r["case"] for r in noisy)
            + " -- the machine was busy; re-run before quoting those."
        )


def explain(record: dict) -> None:
    print(f"{record['status']}: {record['case']} -- {record['description']}")
    for engine in ("mwl", "php"):
        result = record[engine]
        if result.get("failed"):
            print(f"  {engine} exited {result['code']}")
            for line in (result.get("stderr") or "").strip().splitlines()[:12]:
                print(f"    {line}")
    if record["status"] == "DIFF":
        print(f"  mwl printed {record['mwl']['stdout']!r}")
        print(f"  php printed {record['php']['stdout']!r}")


def write_json(path: Path, records: list[dict], binary: Path, php: str, mode: str, reps: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    stamp = time.strftime("%Y-%m-%dT%H:%M:%S")
    commit = version("git", ["rev-parse", "--short", "HEAD"])
    host = {
        "platform": platform.platform(),
        "processor": platform.processor(),
        "cpus": os.cpu_count(),
    }
    with path.open("a", encoding="utf-8", newline="\n") as handle:
        for record in records:
            if record["status"] != "ok":
                continue
            handle.write(json.dumps({
                "date": stamp,
                "commit": commit,
                "case": record["case"],
                "mwl_ms": round(record["mwl"]["min_ms"], 3),
                "php_ms": round(record["php"]["min_ms"], 3),
                "mwl_work_ms": round(record.get("mwl_work_ms", 0.0), 3),
                "php_work_ms": round(record.get("php_work_ms", 0.0), 3),
                "ratio": round(record["ratio"], 4) if "ratio" in record else None,
                "reps": reps,
                "php_mode": mode,
                "mwl_binary": str(binary.relative_to(ROOT)) if binary.is_relative_to(ROOT) else str(binary),
                "host": host,
            }, ensure_ascii=False) + "\n")
    print(f"appended {sum(1 for r in records if r['status'] == 'ok')} record(s) to {path}")


if __name__ == "__main__":
    sys.exit(main())
