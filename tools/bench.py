#!/usr/bin/env python3
"""Run the userland benchmark suite: the same program, once per engine.

    python tools/bench.py                      # every case, every engine, 5 timed reps each
    python tools/bench.py 05 regex             # only cases whose name contains "05" or "regex"
    python tools/bench.py --reps 9             # more reps when a number looks noisy
    python tools/bench.py --check              # correctness only: do they agree? (no timing)
    python tools/bench.py --engines mwl,php    # narrow to two engines; the default is all three
    python tools/bench.py --php-mode default   # PHP as installed, instead of with opcache+JIT
    python tools/bench.py --json docs/perf/userland.ndjson   # append one record per case

The cases live in `benches/userland/` as twins -- `NN-slug.mwl`, `NN-slug.php` and `NN-slug.py`
-- and [its README](../benches/userland/README.md) owns what a case is and how to add one. This
file owns only how they are *measured*.

## Three engines, and why the list is data

`ENGINES` below is a list, not a pair, because the comparison this suite has to answer changed
once: [ADR 0100](../docs/adr/0100-against-python-mwl-claims-the-tool-that-gets-handed-over.md) § 5
adds Python, since MWL's CLI claim is made against Python and this project does not publish an
unmeasured claim. Everything downstream -- the baseline subtraction, the table, the JSON record --
iterates that list, so a fourth engine is an entry plus a file suffix, never a rewrite.

A case is defined by its `.mwl` file. Every other engine attaches if its twin is on disk and is
skipped, with a warning, if it is not: a missing twin must never look like agreement.

## What is measured, and what the number means

Wall-clock of the whole process, `min` over N reps after one discarded warm-up. Minimum rather
than mean because the thing being estimated is the cost of the work, and every source of noise
on a desktop -- a scheduler slice, a page fault, an antivirus scan -- adds time and never
subtracts it. The median is printed beside it so a large min/median gap says "this machine was
busy" out loud rather than hiding inside an average.

`00-baseline` is a case like any other, but its time is also **subtracted** from every other
case in the `work` columns: every engine pays a fixed cost to start a process, read a file and
produce code before a single line of the benchmark runs, and over a 30 ms case that cost is the
measurement. Read `total` for "what does this script cost me at the command line" and `work` for
"how fast is the language". They answer different questions and the suite refuses to pick one --
ADR 0100 § 5 is why `total` is the headline for a CLI claim and `work` for a language claim, and
why quoting either without saying which is a misuse.

The ratio columns are `<engine> / mwl` on the `work` figures: **above 1.0 means MWL is faster**,
and 0.5 means MWL takes twice as long. Each is a ratio of two numbers measured on the same machine
within seconds of each other, which is the only comparison a wall-clock figure supports --
[ADR 0026](../docs/adr/0026-performance-measurement-methodology.md) is why a cross-machine
history is counted in instructions instead, and this suite is that ADR's § 3 secondary figure
rather than a competitor to it.

## Correctness is a gate on the timing, not a separate run

Every case prints a result, and every engine's stdout must match byte for byte before any time is
reported. A benchmark that has quietly stopped doing the same work in one language is worse than
no benchmark, and the cheapest moment to catch it is the run that would otherwise publish its
number. A mismatch prints each output and that case reports `DIFF` with no timing.

## Release against release, always

A debug MWL build is one to two orders of magnitude slower than release and would say nothing
about the language, so a binary under `target/debug/` is refused by name unless `--allow-debug`
says the caller means it. Nothing here builds anything: the binary on disk is the binary that
runs, and a warning says so if it is older than the newest file under `crates/`.

PHP is run with opcache and the tracing JIT on (`--php-mode jit`, the default) because the
project's own target is stated against PHP *with* JIT, and a comparison against an engine that
has been asked to run slower proves nothing. `--php-mode default` runs it exactly as installed
-- the CLI's real out-of-the-box behaviour -- and the mode is recorded in every JSON record, so
the two are never silently mixed in one history file. Python is run as the interpreter running
this script unless `--python` names another, and no flag is passed: there is no second CPython
mode the way there is a second PHP one.

## Adding a measure later

`measure()` returns a dict, `columns_for()` says which keys are printed and how, and everything
else threads dicts around without knowing what is in them. A second measure -- peak RSS, an
allocation count, an instruction count under callgrind -- is a key added there and a column added
here, not a change to the runner.
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
PRIMARY = "mwl"  # the engine every ratio is taken against, and the one that defines a case

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


class Engine:
    """One way to run a case: which twin it reads, how it is invoked, how it names its version.

    ADR 0100 § 5 is why this is a class rather than two hardcoded branches. `label` is what the
    table calls it -- deliberately short, because every engine costs three columns.
    """

    def __init__(self, key: str, label: str, suffix: str, argv: list[str], version_argv: list[str]) -> None:
        self.key = key
        self.label = label
        self.suffix = suffix
        self.argv = argv
        self.version_argv = version_argv

    def command(self, source: Path) -> list[str]:
        return [*self.argv, str(source)]


class Case:
    """One benchmark: the same program written once per engine, and what it is for."""

    def __init__(self, name: str, sources: dict[str, Path], description: str) -> None:
        self.name = name
        self.sources = sources
        self.description = description

    @property
    def is_baseline(self) -> bool:
        return self.name == BASELINE


def build_engines(selected: list[str], mwl_binary: Path, php: str, php_mode: str, python: str) -> list[Engine]:
    """The engine roster, in table order. `--engines` picks a subset; `mwl` is always in it."""
    available = {
        "mwl": Engine("mwl", "mwl", ".mwl", [str(mwl_binary), "run"], ["--version"]),
        "php": Engine("php", "php", ".php", [php, *PHP_MODES[php_mode]], ["-r", "echo PHP_VERSION;"]),
        "python": Engine("python", "py", ".py", [python], ["--version"]),
    }
    unknown = [key for key in selected if key not in available]
    if unknown:
        sys.exit(f"unknown engine(s) {unknown}; known: {', '.join(available)}")
    if PRIMARY not in selected:
        sys.exit(f"--engines must include {PRIMARY}: every ratio is taken against it")
    return [available[key] for key in selected]


def discover(patterns: list[str], engines: list[Engine]) -> list[Case]:
    """Every `NN-slug.mwl`, in name order, with whichever twins are beside it.

    A case is defined by its `.mwl`. An engine whose twin is missing is skipped for that case and
    said so out loud -- silence would read as agreement.
    """
    cases = []
    for mwl in sorted(CASE_DIR.glob("*.mwl")):
        sources = {}
        for engine in engines:
            source = mwl.with_suffix(engine.suffix)
            if source.exists():
                sources[engine.key] = source
            else:
                print(f"warning: {mwl.stem} has no {engine.suffix} twin -- {engine.key} skipped", file=sys.stderr)
        if len(sources) < 2:
            print(f"warning: {mwl.stem} has nothing to compare against -- skipped", file=sys.stderr)
            continue
        cases.append(Case(mwl.stem, sources, describe(mwl)))
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
    here and another entry in `columns_for()`. Nothing between the two needs to know it exists.
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


def columns_for(engines: list[Engine]) -> list[tuple[str, int, object]]:
    """header, width, how to read one row's value out of the joined record.

    Total time per engine, then work time per engine, then one ratio per engine but the primary --
    grouped by measure rather than by engine so a column reads down as a comparison.
    """
    columns: list[tuple[str, int, object]] = []
    for engine in engines:
        columns.append((f"{engine.label} ms", 10, lambda r, k=engine.key: fmt(r["engines"].get(k, {}).get("min_ms"))))
    for engine in engines:
        columns.append((f"{engine.label} work", 11, lambda r, k=engine.key: fmt(r.get("work", {}).get(k))))
    for engine in engines:
        if engine.key == PRIMARY:
            continue
        columns.append((f"{engine.label}/mwl", 10, lambda r, k=engine.key: ratio(r.get("ratio", {}).get(k))))
    return columns


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:,.1f}"


def ratio(value: float | None) -> str:
    if value is None:
        return "-"
    return f"{value:.2f}x"


def evaluate(case: Case, engines: list[Engine], reps: int) -> dict:
    """Run every engine that has a twin for this case, and gate the timing on their agreement."""
    results = {}
    for engine in engines:
        source = case.sources.get(engine.key)
        if source is not None:
            results[engine.key] = measure(engine.command(source), reps)

    record = {"case": case.name, "description": case.description, "engines": results}
    outputs = {key: r["stdout"] for key, r in results.items() if not r["failed"]}
    if any(r["failed"] for r in results.values()):
        record["status"] = "ERROR"
    elif len(set(outputs.values())) > 1:
        record["status"] = "DIFF"
    else:
        record["status"] = "ok"
    return record


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run the userland benchmark suite across MWL, PHP and Python side by side.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__.split("## Three engines", 1)[0],
    )
    parser.add_argument("patterns", nargs="*", help="substrings; only matching cases run")
    parser.add_argument("--reps", type=int, default=5, help="timed reps per engine (default 5)")
    parser.add_argument("--check", action="store_true", help="agreement only, no timing")
    parser.add_argument("--mwl", help="path to the mwl binary (default target/release)")
    parser.add_argument("--php", default="php", help="php executable (default `php`)")
    parser.add_argument(
        "--python",
        default=sys.executable,
        help="python executable (default: the one running this script)",
    )
    parser.add_argument(
        "--engines",
        default="mwl,php,python",
        help="comma-separated engine list, in table order; must include mwl (default all three)",
    )
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
    selected = [name.strip() for name in args.engines.split(",") if name.strip()]
    engines = build_engines(selected, binary, args.php, args.php_mode, args.python)
    cases = discover(args.patterns, engines)
    if not cases:
        sys.exit("no runnable case found under benches/userland")

    for engine in engines:
        target = binary if engine.key == PRIMARY else engine.argv[0]
        mode = f" [{args.php_mode}]" if engine.key == "php" else ""
        print(f"{engine.label:<4} {target}{mode}  ({version(target, engine.version_argv)})")
    print(
        f"{len(cases)} case(s), {reps} rep(s), min of reps;"
        f" a ratio above 1.00 means MWL is faster"
    )
    print()

    records = []
    for case in cases:
        record = evaluate(case, engines, reps)
        records.append(record)
        print(f"  {record['status']:<5} {case.name}", flush=True)

    subtract_baseline(records, engines)

    print()
    if args.check:
        report_check(records)
    else:
        report_table(records, engines)

    if args.json:
        write_json(Path(args.json), records, engines, binary, args.php_mode, reps)

    return 0 if all(r["status"] == "ok" for r in records) else 1


def subtract_baseline(records: list[dict], engines: list[Engine]) -> None:
    """`work` is `total` minus the empty program, per engine; the ratio is over `work`."""
    baseline = next((r for r in records if r["case"] == BASELINE and r["status"] == "ok"), None)
    if not baseline:
        return
    for record in records:
        if record["status"] != "ok":
            continue
        work = {}
        for engine in engines:
            mine = record["engines"].get(engine.key)
            floor = baseline["engines"].get(engine.key)
            if mine and floor and not mine["failed"] and not floor["failed"]:
                work[engine.key] = max(0.0, mine["min_ms"] - floor["min_ms"])
        record["work"] = work
        primary = work.get(PRIMARY)
        if primary:
            record["ratio"] = {k: v / primary for k, v in work.items() if k != PRIMARY}


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
            agreed = next(iter(record["engines"].values()))["stdout"].strip()
            engines = "+".join(record["engines"])
            print(f"ok    {record['case']}  [{engines}]  ->  {agreed!r}")
        else:
            explain(record)


def report_table(records: list[dict], engines: list[Engine]) -> None:
    columns = columns_for(engines)
    width = max(len(r["case"]) for r in records)
    header = "case".ljust(width) + "".join(h.rjust(w) for h, w, _ in columns)
    print(header)
    print("-" * len(header))
    for record in records:
        if record["status"] != "ok":
            print(record["case"].ljust(width) + record["status"].rjust(10))
            continue
        print(record["case"].ljust(width) + "".join(read(record).rjust(w) for _, w, read in columns))
    print("-" * len(header))
    for engine in engines:
        if engine.key == PRIMARY:
            continue
        ratios = [
            (r["case"], r["ratio"][engine.key])
            for r in records
            if r["status"] == "ok" and engine.key in r.get("ratio", {})
        ]
        if not ratios:
            continue
        best = max(ratios, key=lambda pair: pair[1])
        worst = min(ratios, key=lambda pair: pair[1])
        print(
            f"vs {engine.label:<4} best {best[0]} {best[1]:.2f}x |"
            f" worst {worst[0]} {worst[1]:.2f}x |"
            f" median {statistics.median(v for _, v in ratios):.2f}x"
        )
    for record in records:
        if record["status"] != "ok":
            print()
            explain(record)
    noisy = [
        r for r in records
        if r["status"] == "ok"
        and any(e["median_ms"] > 1.25 * e["min_ms"] for e in r["engines"].values() if not e["failed"])
    ]
    if noisy:
        print(
            "note: min and median differ by more than 25% on "
            + ", ".join(r["case"] for r in noisy)
            + " -- the machine was busy; re-run before quoting those."
        )


def explain(record: dict) -> None:
    print(f"{record['status']}: {record['case']} -- {record['description']}")
    for key, result in record["engines"].items():
        if result.get("failed"):
            print(f"  {key} exited {result['code']}")
            for line in (result.get("stderr") or "").strip().splitlines()[:12]:
                print(f"    {line}")
    if record["status"] == "DIFF":
        for key, result in record["engines"].items():
            if not result.get("failed"):
                print(f"  {key} printed {result['stdout']!r}")


def write_json(
    path: Path, records: list[dict], engines: list[Engine], binary: Path, mode: str, reps: int
) -> None:
    """One NDJSON record per case.

    The `mwl_ms`/`php_ms`/`ratio` keys are kept at the top level unchanged so the history file
    written before this suite grew a third engine still reads as one series; everything else is
    under `engines`, keyed the same way the table is.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    stamp = time.strftime("%Y-%m-%dT%H:%M:%S")
    commit = version("git", ["rev-parse", "--short", "HEAD"])
    host = {
        "platform": platform.platform(),
        "processor": platform.processor(),
        "cpus": os.cpu_count(),
    }
    versions = {
        e.key: version(binary if e.key == PRIMARY else e.argv[0], e.version_argv) for e in engines
    }
    written = 0
    with path.open("a", encoding="utf-8", newline="\n") as handle:
        for record in records:
            if record["status"] != "ok":
                continue
            per_engine = {
                key: {
                    "ms": round(result["min_ms"], 3),
                    "work_ms": round(record.get("work", {}).get(key, 0.0), 3),
                    "ratio": round(record["ratio"][key], 4) if key in record.get("ratio", {}) else None,
                }
                for key, result in record["engines"].items()
                if not result["failed"]
            }
            handle.write(json.dumps({
                "date": stamp,
                "commit": commit,
                "case": record["case"],
                "mwl_ms": per_engine.get("mwl", {}).get("ms"),
                "php_ms": per_engine.get("php", {}).get("ms"),
                "mwl_work_ms": per_engine.get("mwl", {}).get("work_ms"),
                "php_work_ms": per_engine.get("php", {}).get("work_ms"),
                "ratio": per_engine.get("php", {}).get("ratio"),
                "engines": per_engine,
                "engine_versions": versions,
                "reps": reps,
                "php_mode": mode,
                "mwl_binary": str(binary.relative_to(ROOT)) if binary.is_relative_to(ROOT) else str(binary),
                "host": host,
            }, ensure_ascii=False) + "\n")
            written += 1
    print(f"appended {written} record(s) to {path}")


if __name__ == "__main__":
    sys.exit(main())
