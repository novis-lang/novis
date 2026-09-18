#!/usr/bin/env python3
"""Run the userland benchmark suite: the same program, once per engine.

    python tools/bench.py                      # every case, every engine, 5 timed reps each
    python tools/bench.py 05 regex             # only cases whose name contains "05" or "regex"
    python tools/bench.py --reps 9             # more reps when a number looks noisy
    python tools/bench.py --check              # correctness only: do they agree? (no timing)
    python tools/bench.py --engines nvs,php    # narrow the roster; the default is all four
    python tools/bench.py --php-mode default   # PHP as installed, instead of with opcache+JIT
    python tools/bench.py --json docs/perf/userland.ndjson   # append one record per case
    python tools/bench.py --warm-start --max-work-ms 6   # the CLI's own start cost, budgeted

The cases live in `benches/userland/` as twins -- `NN-slug.nvs`, `.php`, `.py` and `.ts` -- and
[its README](../benches/userland/README.md) owns what a case is and how to add one. This file
owns only how they are *measured*.

## The engine roster, and why the list is data

The roster in `build_engines()` is a list, not a pair, because the comparison this suite has to
answer keeps changing:
`rule:tooling/bench-engine-list-is-data` added
Python, since Novis's CLI claim is made against Python and this project does not publish an
unmeasured claim, and that ADR's *Revisiting* is what pre-authorised Bun as the fourth. Everything
downstream -- the baseline subtraction, the table, the JSON record -- iterates the list, so a
fifth engine is an entry plus a file suffix, never a rewrite.

A case is defined by its `.nvs` file. Every other engine attaches if its twin is on disk and is
skipped, with a warning, if it is not: a missing twin must never look like agreement. An engine
whose executable is not installed fails every case it is asked to run, so name the ones you have
with `--engines` rather than reading a wall of ERROR rows.

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
`rule:tooling/bench-engine-list-is-data` is why `total` is the headline for a CLI claim and `work` for a language claim, and
why quoting either without saying which is a misuse.

The ratio columns are `<engine> / nvs` on the `work` figures: **above 1.0 means Novis is faster**,
and 0.5 means Novis takes twice as long. Each is a ratio of two numbers measured on the same machine
within seconds of each other, which is the only comparison a wall-clock figure supports --
`rule:testing/perf-two-mechanisms` is why a cross-machine
history is counted in instructions instead, and this suite is that ADR's § 3 secondary figure
rather than a competitor to it.

## Correctness is a gate on the timing, not a separate run

Every case prints a result, and every engine's stdout must match byte for byte before any time is
reported. A benchmark that has quietly stopped doing the same work in one language is worse than
no benchmark, and the cheapest moment to catch it is the run that would otherwise publish its
number. A mismatch prints each output and that case reports `DIFF` with no timing.

## Release against release, always

A debug Novis build is one to two orders of magnitude slower than release and would say nothing
about the language, so a binary under `target/debug/` is refused by name unless `--allow-debug`
says the caller means it. Nothing here builds anything: the binary on disk is the binary that
runs, and a warning says so if it is older than the newest file under `crates/`.

PHP is run with opcache and the tracing JIT on (`--php-mode jit`, the default) because the
project's own target is stated against PHP *with* JIT, and a comparison against an engine that
has been asked to run slower proves nothing. `--php-mode default` runs it exactly as installed
-- the CLI's real out-of-the-box behaviour -- and the mode is recorded in every JSON record, so
the two are never silently mixed in one history file. Python is run as the interpreter running
this script unless `--python` names another, and no flag is passed: there is no second CPython
mode the way there is a second PHP one. Bun runs the `.ts` file directly -- it transpiles
TypeScript on the way in, which is part of what a Bun user pays at start-up and so is deliberately
inside the measurement rather than pre-compiled away.

## The warm-start figure

`--warm-start` answers a different question from the table: not "how fast is the language" but
"what does the CLI cost me before it has done anything", which is M6's own acceptance figure. It is
one engine, one script -- the baseline case -- and no comparison, so it runs whether or not PHP,
Python or Bun is installed. What makes it *warm* is the process `measure()` already discards: by
the timed reps the binary, its libraries and the script are in the OS page cache, and the artifact
that run published is on disk for `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` to load. `warm_start()` owns both halves.

What `--max-work-ms` budgets is the total **less** `nvs --version`, because most of the total is
the operating system creating a process and not Novis at all. A machine whose floor is itself
multiples of the budget is dilating both halves and reports as not measured rather than as a
regression. `warm_start()` owns both arguments and the measurements behind them.

## The serve-versus-FPM leg

`--serve-vs-fpm` answers M7's own *Verify* line: requests/sec for `nvs serve` against PHP 8.5 with
opcache, recorded in `benches/`. That paragraph names `wrk`/`oha` and PHP-FPM, and **neither is on
the boxes this repository is developed and tested on** -- Windows has no php-fpm at all, and no load
generator is installed. Three ways out were open and the choice is recorded here rather than
re-argued every time the leg is read:

- **The generator is this file**, not `wrk`. A closed-loop generator over keep-alive connections is
  a page of `socket`, it is the same page on every platform, and a benchmark that runs only where a
  C tool happens to be installed is a benchmark that never runs. `wrk` measures the same quantity
  with better tail statistics; this leg reports no tail, so the difference does not arise.
- **The baseline is `php-cgi -b`, and that is not a stand-in for FPM -- it is the same SAPI.** FPM
  is the CGI/FastCGI SAPI plus a process manager, and `php-cgi -b host:port` is that SAPI speaking
  that protocol, which is exactly how PHP is deployed on Windows where FPM does not exist. It is
  driven over FastCGI with opcache on, as nginx would drive FPM -- so PHP pays no HTTP parse and no
  proxy hop while `nvs serve` pays both. The comparison is therefore **biased towards PHP**, which
  is the only direction a project may bias a benchmark of itself. `php -S` is the fallback when
  `php-cgi` is missing; it is named as such in the record and it is the worse peer, being a
  documented development server with no process manager behind it.
- **Nothing is refused.** With no PHP at all the leg measures `nvs serve` alone and records
  `baseline: null`. A hosted runner has no PHP, and a check that went red there would be reporting
  the runner rather than the tree.

**Concurrency defaults to 1, and the baseline is why.** Neither peer this box can offer serves two
requests at once: `php-cgi -b` is one process with no `PHP_FCGI_CHILDREN` to fork it, and `php -S`
answers serially by construction. Past 1, `--concurrency` measures Novis against a queue rather than
against PHP and flatters it by however deep the queue got. The flag exists for a machine with a real
FPM pool, and the value is in every record, so a figure taken at 1 and one taken at 64 can never be
read as one series.

Connections are opened before the clock starts and the handshake is outside the measurement: the two
peers disagree about how many handshakes a run needs, and that difference is not what is being
compared. Both bodies must match byte for byte before either number is reported, for the reason the
suite's own agreement gate exists.

`--record PATH` appends one object to a JSON array -- `benches/serve.json` is the path M7 names, and
`write_serve_record` says why the loop's own sweep points it somewhere else instead. The
whole run is one object: both peers, both versions, the concurrency, the request count and the
caveats that applied, because a requests/sec figure with no peer written beside it measures nothing.

## Adding a measure later

`measure()` returns a dict, `columns_for()` says which keys are printed and how, and everything
else threads dicts around without knowing what is in them. A second measure -- peak RSS, an
allocation count, an instruction count under callgrind -- is a key added there and a column added
here, not a change to the runner.
"""

from __future__ import annotations

import argparse
import collections
import json
import os
import platform
import shutil
import socket
import statistics
import struct
import subprocess
import sys
import threading
import time
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):  # a case description is prose, and consoles here are cp1252
    sys.stdout.reconfigure(encoding="utf-8")

ROOT = Path(__file__).resolve().parent.parent
CASE_DIR = ROOT / "benches" / "userland"
BASELINE = "00-baseline"
PRIMARY = "nvs"  # the engine every ratio is taken against, and the one that defines a case
QUIET_FLOOR_MS = 5.75  # `warm_start` abstains above this start floor; its note is why
SUITE_REPS = 5  # timed reps per engine per case, over a case that runs for tens of ms
WARM_START_REPS = 25  # the warm-start leg estimates a difference of two small numbers; its note is why

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

    `rule:tooling/bench-engine-list-is-data` is why this is a class rather than two hardcoded branches. `label` is what the
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


def build_engines(
    selected: list[str], nvs_binary: Path, php: str, php_mode: str, python: str, bun: str
) -> list[Engine]:
    """The engine roster, in table order. `--engines` picks a subset; `nvs` is always in it."""
    available = {
        "nvs": Engine("nvs", "nvs", ".nvs", [str(nvs_binary), "run"], ["--version"]),
        "php": Engine("php", "php", ".php", [php, *PHP_MODES[php_mode]], ["-r", "echo PHP_VERSION;"]),
        "python": Engine("python", "py", ".py", [python], ["--version"]),
        "bun": Engine("bun", "bun", ".ts", [bun, "run"], ["--version"]),
    }
    unknown = [key for key in selected if key not in available]
    if unknown:
        sys.exit(f"unknown engine(s) {unknown}; known: {', '.join(available)}")
    if PRIMARY not in selected:
        sys.exit(f"--engines must include {PRIMARY}: every ratio is taken against it")
    return [available[key] for key in selected]


def discover(patterns: list[str], engines: list[Engine]) -> list[Case]:
    """Every `NN-slug.nvs`, in name order, with whichever twins are beside it.

    A case is defined by its `.nvs`. An engine whose twin is missing is skipped for that case and
    said so out loud -- silence would read as agreement.
    """
    cases = []
    for nvs in sorted(CASE_DIR.glob("*.nvs")):
        sources = {}
        for engine in engines:
            source = nvs.with_suffix(engine.suffix)
            if source.exists():
                sources[engine.key] = source
            else:
                print(f"warning: {nvs.stem} has no {engine.suffix} twin -- {engine.key} skipped", file=sys.stderr)
        if len(sources) < 2:
            print(f"warning: {nvs.stem} has nothing to compare against -- skipped", file=sys.stderr)
            continue
        cases.append(Case(nvs.stem, sources, describe(nvs)))
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


def find_nvs(explicit: str | None, allow_debug: bool) -> Path:
    if explicit:
        binary = Path(explicit)
    else:
        exe = "nvs.exe" if os.name == "nt" else "nvs"
        binary = ROOT / "target" / "release" / exe
    if not binary.exists():
        sys.exit(
            f"no Novis binary at {binary}\n"
            "  build one with `cargo build --release`, or point --nvs at the one you mean"
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


def warm_start(binary: Path, reps: int, max_work_ms: float | None) -> int:
    """What a *second* `nvs run` of the empty program costs, less what starting any process costs.

    Two measurements rather than one, because the wall clock of the whole process is mostly not
    Novis. On the 2026-09-04 box `nvs run` of the baseline case was 11.0 ms and `nvs --version`
    alone was 6.4 ms: 58% of the figure was the operating system creating a process, and the same
    6.4-6.9 ms was paid by a 24 MB test binary out of this workspace and by this binary copied off
    the repository's drive entirely. **Budgeting the total budgets the machine**, and it was
    measured doing exactly that -- 8.3-9.2 ms across forty sweeps and then 10.6-11.2 ms with no
    compiled change but a doc comment, two workspace binaries built ninety minutes either side of
    the step both starting in 6.9 ms, and the cause never found on the box.

    So `--max-work-ms` budgets the DIFFERENCE, which is the part Novis owns and the part a
    regression appears in. `--version` is the right floor rather than some other program because it
    is the same binary, the same loader work and the same page cache, so what the subtraction
    leaves is config load, compile and run and nothing else.

    That subtraction is a measurement only while the machine runs at the speed the budget was
    written against, because load here dilates a process *multiplicatively* rather than adding a
    constant to it: one sweep read a 77.5 ms start floor where a quiet box reads 4.6, and the
    difference it left was 200 ms of machine and no Novis at all. Spread does not tell that box
    from a good one -- its median sat 4% over its minimum, as tight as any quiet run, because every
    rep was equally slow. The *level* of the floor does, and it is the reading to use because
    `nvs --version` is this binary doing strictly less than the budget covers: above
    `QUIET_FLOOR_MS` the machine cannot answer the question, so the guard reports itself not
    measured and passes rather than reporting the machine as the tree. `benches/abi-probe`'s
    fan-out guard abstains on its own control for the same reason.

    That threshold is a floor level and not a multiple of the budget, because the two measure
    different things and only the floor moves with the machine. Anchored to the budget it abstained
    at 24 ms, which is five times what a quiet box reads, so it saw the 77.5 ms box and nothing
    milder: an acceptance sweep leaves a shadow that outlasts the driver's own `COST_SETTLE`, and
    one read a 6.1 ms floor and 7.1 ms of work thirty seconds after finishing, where the same
    binary idle reads 4.5 and 5.5. Both asks were red, so the driver called the shadow a
    regression. A millisecond budget is meaningful only on a box at least as quick as the one it
    was written against, and the floor is how this leg checks that before it believes a
    difference -- a box whose quiet floor is above `QUIET_FLOOR_MS` never measures here rather
    than failing on its own speed.

    This leg also takes `WARM_START_REPS` where the suite takes `SUITE_REPS`, because a minimum
    over a handful of samples is a biased-high estimate of a cost, and here that bias lands on a
    difference of two small numbers rather than on a total of tens of milliseconds. Idle, five reps
    moved the figure a full millisecond between consecutive runs -- more than the budget has to give
    -- and twenty-five held it inside three tenths, with the floor reading the same to a tenth every
    time. A rep costs about as long as the thing it measures, so the accuracy is nearly free.

    What that cannot see is a regression in the floor itself: a static initializer that quadrupled
    `nvs --version` would silence this guard rather than trip it. Both figures are printed on every
    run, green or not, so the one it is blind to is still on the page.

    `measure()` throws its first process away, and that discarded run is what makes this warm in
    both senses. The binary, its libraries and the script are in the OS page cache for every timed
    rep; and the discarded run published the artifact
    `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` then has each timed rep load,
    so the figure covers the read path -- verify, place, relocate, protect, bind -- and not a
    compile. A cache directory § 5 refuses is the one case where it does not: every rep compiles
    then, which is the shape a regression here takes rather than an error anyone sees.
    """
    source = CASE_DIR / f"{BASELINE}.nvs"
    if not source.exists():
        sys.exit(f"no {source}: the warm-start figure is measured on the baseline case")

    print(f"nvs  {binary}  ({version(binary, ['--version'])})")
    floor = measure([str(binary), "--version"], reps)
    total = measure([str(binary), "run", str(source)], reps)
    for what, result in (("nvs --version", floor), ("nvs run", total)):
        if result["failed"]:
            print(f"warm start FAILED: `{what}` exit {result['code']}")
            print(result["stderr"].rstrip() or result["stdout"].rstrip(), file=sys.stderr)
            return 1

    work_ms = total["min_ms"] - floor["min_ms"]
    label = source.relative_to(ROOT).as_posix()
    print(f"warm start  nvs run {label}  ({reps} rep(s), one discarded warm-up)")
    print(f"  start floor {fmt(floor['min_ms'])} ms   nvs --version, the OS creating a process")
    print(f"  total       {fmt(total['min_ms'])} ms   median {fmt(total['median_ms'])} ms")
    print(f"  novis work  {fmt(work_ms)} ms   the total less that floor")
    print("  each rep is an `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` warm hit; the discarded warm-up published the artifact")
    if max_work_ms is None:
        return 0
    if floor["min_ms"] > QUIET_FLOOR_MS:
        print(f"  not measured -- `nvs --version` alone costs {fmt(floor['min_ms'])} ms here, over "
              f"the {fmt(QUIET_FLOOR_MS)} ms a box this budget can be read on, so the "
              f"{fmt(work_ms)} ms difference is a figure about the machine")
        return 0
    within = work_ms <= max_work_ms
    print(f"  budget {fmt(max_work_ms)} ms on the work -- {'within' if within else 'EXCEEDED'}")
    return 0 if within else 1


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
        columns.append((f"{engine.label}/nvs", 10, lambda r, k=engine.key: ratio(r.get("ratio", {}).get(k))))
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


# --------------------------------------------------------------------------------------
# The serve-versus-FPM leg. The module doc's section of that name owns every decision
# below -- which peer, why the generator is here, why concurrency is 1; this half owns
# only how it is carried out.
# --------------------------------------------------------------------------------------

SERVE_DIR = ROOT / "benches" / "serve"
SERVE_CASE = "hello"  # the twin pair under SERVE_DIR; its README-less smallness is the point
SERVE_HISTORY = 100  # runs kept in `--record`'s artifact; `write_serve_record` says why it is capped

# FastCGI 1.0 record types and the two constants a responder request needs. Six numbers is
# the whole of the protocol used here, which is why there is a client below and not a
# dependency.
FCGI_BEGIN_REQUEST, FCGI_END_REQUEST = 1, 3
FCGI_PARAMS, FCGI_STDIN, FCGI_STDOUT, FCGI_STDERR = 4, 5, 6, 7
FCGI_RESPONDER, FCGI_KEEP_CONN = 1, 1


def free_port() -> int:
    """A port the OS has just said is free. Racy by nature, and every harness is."""
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


class HttpConn:
    """One keep-alive HTTP/1.1 connection, issuing one GET at a time.

    Reopened transparently when the peer answers `Connection: close` -- `php -S` does, on
    every response -- and `reconnects` counts it, for `FcgiConn`'s reason: a peer paying a
    handshake per request and one that is not are two different measurements.
    """

    def __init__(self, port: int, path: str = "/") -> None:
        self.port = port
        self.wire = f"GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: */*\r\n\r\n".encode()
        self.reconnects = 0
        self.sock = None
        self._connect()

    def _connect(self) -> None:
        self.sock = socket.create_connection(("127.0.0.1", self.port), 10)
        self.sock.settimeout(30)
        self.buf = b""

    def _fill(self) -> None:
        chunk = self.sock.recv(65536)
        if not chunk:
            raise ConnectionError("the server closed the connection mid-response")
        self.buf += chunk

    def request(self) -> bytes:
        try:
            return self._exchange()
        except OSError:  # the peer answered `Connection: close`; that is a measurement
            self.reconnects += 1
            self.sock.close()
            self._connect()
            return self._exchange()

    def _exchange(self) -> bytes:
        self.sock.sendall(self.wire)
        while b"\r\n\r\n" not in self.buf:
            self._fill()
        head, self.buf = self.buf.split(b"\r\n\r\n", 1)
        lines = head.lower().split(b"\r\n")
        if any(line.startswith(b"transfer-encoding:") for line in lines):
            raise RuntimeError("a chunked response is not measured here; the case sends a fixed body")
        length = next(
            (int(line.split(b":", 1)[1]) for line in lines if line.startswith(b"content-length:")),
            None,
        )
        closing = any(line == b"connection: close" for line in lines)
        if length is not None:
            while len(self.buf) < length:
                self._fill()
            body, self.buf = self.buf[:length], self.buf[length:]
        elif closing:
            # HTTP/1.0 framing: the body ends when the peer closes, which is what `php -S`
            # sends. Reading to EOF is the only way to see the bytes the agreement gate needs.
            while True:
                chunk = self.sock.recv(65536)
                if not chunk:
                    break
                self.buf += chunk
            body, self.buf = self.buf, b""
        else:
            raise RuntimeError("a response framed by neither Content-Length nor a close is not read here")
        if closing:
            # Reopened here rather than by failing the next request into a dead socket: the
            # peer asked for a handshake per request and the honest thing is to pay it once.
            self.reconnects += 1
            self.sock.close()
            self._connect()
        return body

    def close(self) -> None:
        self.sock.close()


def _fcgi_len(n: int) -> bytes:
    return bytes([n]) if n < 128 else struct.pack(">I", n | 0x80000000)


def _fcgi_pair(name: str, value: str) -> bytes:
    key, val = name.encode(), value.encode()
    return _fcgi_len(len(key)) + _fcgi_len(len(val)) + key + val


def _fcgi_record(kind: int, body: bytes, request_id: int = 1) -> bytes:
    return struct.pack(">BBHHBB", 1, kind, request_id, len(body), 0, 0) + body


class FcgiConn:
    """One FastCGI connection to `php-cgi -b`, issuing one responder request at a time.

    `FCGI_KEEP_CONN` is set, so the connection is reused where the SAPI honours it and
    reopened transparently where it does not. `reconnects` counts the second case and the
    record carries it: a peer paying a TCP handshake per request and one that is not are two
    different measurements, and the artifact has to say which was taken.
    """

    def __init__(self, port: int, script: Path) -> None:
        self.port = port
        self.reconnects = 0
        self.params = b"".join(_fcgi_pair(k, v) for k, v in {
            "GATEWAY_INTERFACE": "CGI/1.1",
            "REQUEST_METHOD": "GET",
            "SCRIPT_FILENAME": str(script),
            "SCRIPT_NAME": "/" + script.name,
            "REQUEST_URI": "/" + script.name,
            "DOCUMENT_ROOT": str(script.parent),
            "QUERY_STRING": "",
            "SERVER_PROTOCOL": "HTTP/1.1",
            "SERVER_SOFTWARE": "novis-bench",
            "SERVER_NAME": "127.0.0.1",
            "REMOTE_ADDR": "127.0.0.1",
            "CONTENT_LENGTH": "0",
        }.items())
        self.sock = None
        self._connect()

    def _connect(self) -> None:
        self.sock = socket.create_connection(("127.0.0.1", self.port), 10)
        self.sock.settimeout(30)

    def _exactly(self, n: int) -> bytes:
        out = b""
        while len(out) < n:
            chunk = self.sock.recv(n - len(out))
            if not chunk:
                raise ConnectionError("php-cgi closed the connection mid-record")
            out += chunk
        return out

    def request(self) -> bytes:
        try:
            return self._exchange()
        except OSError:  # the SAPI declined FCGI_KEEP_CONN; that is a measurement, not a failure
            self.reconnects += 1
            self._connect()
            return self._exchange()

    def _exchange(self) -> bytes:
        self.sock.sendall(
            _fcgi_record(FCGI_BEGIN_REQUEST, struct.pack(">HB5x", FCGI_RESPONDER, FCGI_KEEP_CONN))
            + _fcgi_record(FCGI_PARAMS, self.params)
            + _fcgi_record(FCGI_PARAMS, b"")
            + _fcgi_record(FCGI_STDIN, b"")
        )
        out, err = b"", b""
        while True:
            _, kind, _, length, padding, _ = struct.unpack(">BBHHBB", self._exactly(8))
            body = self._exactly(length + padding)[:length]
            if kind == FCGI_STDOUT:
                out += body
            elif kind == FCGI_STDERR:
                err += body
            elif kind == FCGI_END_REQUEST:
                break
        if err:
            raise RuntimeError("php-cgi wrote to stderr: " + err.decode(errors="replace")[:300])
        return out.split(b"\r\n\r\n", 1)[1] if b"\r\n\r\n" in out else out

    def close(self) -> None:
        self.sock.close()


class Server:
    """A server subprocess, listening, with its output drained.

    The drain thread is not tidiness: `php -S` logs a line per request, and an undrained
    pipe wedges it at the OS buffer size -- which reads as the peer becoming slow, halfway
    through a run, for no reason the numbers can explain.
    """

    def __init__(self, argv: list[str], port: int, cwd: Path | None = None,
                 env: dict | None = None) -> None:
        self.argv = argv
        self.log: collections.deque = collections.deque(maxlen=60)
        self.proc = subprocess.Popen(
            argv,
            cwd=str(cwd) if cwd else None,
            env={**os.environ, **env} if env else None,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            errors="replace",
        )
        threading.Thread(target=self._drain, daemon=True).start()
        self._await(port)

    def _drain(self) -> None:
        for line in self.proc.stdout:
            self.log.append(line.rstrip())

    def said(self) -> str:
        return "\n".join(f"    {line}" for line in self.log) or "    (nothing)"

    def _await(self, port: int) -> None:
        deadline = time.time() + 30
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(
                    f"{self.argv[0]} exited {self.proc.returncode} before it listened:\n{self.said()}"
                )
            try:
                socket.create_connection(("127.0.0.1", port), 0.5).close()
                return
            except OSError:
                time.sleep(0.05)
        self.stop()
        raise RuntimeError(f"{self.argv[0]} did not listen on port {port} within 30s:\n{self.said()}")

    def stop(self) -> None:
        self.proc.terminate()
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait()


def hammer(open_conn, requests: int, concurrency: int) -> tuple[float, bytes]:
    """`requests` GETs over `concurrency` keep-alive connections: seconds, and the body.

    Closed-loop -- each connection issues its next request the moment the previous answer is
    in hand, which is what a client-side requests/sec figure means. The connections are
    opened before the clock starts; the module doc says why the handshake is outside.
    """
    share = [requests // concurrency + (1 if i < requests % concurrency else 0)
             for i in range(concurrency)]
    conns = [open_conn() for _ in share]
    bodies: list[bytes] = [b""] * concurrency
    errors: list[BaseException | None] = [None] * concurrency

    def run(index: int) -> None:
        try:
            for _ in range(share[index]):
                bodies[index] = conns[index].request()
        except BaseException as exc:  # carried out of the thread, never swallowed
            errors[index] = exc

    threads = [threading.Thread(target=run, args=(i,)) for i in range(concurrency)]
    start = time.perf_counter()
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    elapsed = time.perf_counter() - start
    for conn in conns:
        try:
            conn.close()
        except OSError:
            pass
    for exc in errors:
        if exc is not None:
            raise exc
    answered = {body for body in bodies if body}
    if len(answered) > 1:
        raise RuntimeError("one server answered two connections with two different bodies")
    reopened = sum(getattr(conn, "reconnects", 0) for conn in conns)
    return elapsed, next(iter(answered), b""), reopened


def measure_server(peer: dict, requests: int, concurrency: int, reps: int) -> dict:
    """Boot one peer, warm it, time `reps` passes over it, and shut it down.

    A failure mid-run is re-raised carrying what the server printed: the generator only ever
    sees a reset socket, and the sentence that says why is always on the peer's own stdout.
    """
    server = Server(peer["argv"], peer["port"], peer.get("cwd"), peer.get("env"))
    try:
        # Discarded: the page cache, opcache's first compile and the JIT's first trace are
        # start-up costs, and this leg is measuring a server that has been up for a while.
        hammer(peer["open"], max(20, min(requests, 200)), concurrency)
        rates, body, reopened = [], b"", 0
        for _ in range(reps):
            elapsed, body, reconnects = hammer(peer["open"], requests, concurrency)
            rates.append(requests / elapsed)
            reopened += reconnects
    except Exception as exc:
        hint = (
            f"; at --concurrency {concurrency} a single-process peer does not answer the second "
            "connection at all, which arrives here as a timeout -- the module doc says why 1 is "
            "the default" if concurrency > 1 else ""
        )
        raise RuntimeError(
            f"{peer['label']} failed mid-run ({exc}){hint}; it said:\n{server.said()}"
        ) from exc
    finally:
        server.stop()
    best = max(rates)
    return {
        "reconnects": reopened,
        "kind": peer["kind"],
        "label": peer["label"],
        "detail": peer["detail"],
        "requests_per_sec": best,
        "median_requests_per_sec": statistics.median(rates),
        "ms_per_request": 1000.0 * concurrency / best,
        "version": version(peer["executable"], peer["version_argv"]),
        "body": body,
    }


def php_peer(php: str, php_mode: str, choice: str = "auto") -> dict | None:
    """The best PHP peer this box can offer, in the order the module doc argues for.

    `choice` names one instead of taking the best: `--serve-baseline` exists so the two
    weaker branches are *reachable* on a box that has the strong one. A fallback nothing can
    run is a fallback nobody has run, and it fails the first time it is needed.
    """
    script = SERVE_DIR / f"{SERVE_CASE}.php"
    opcache = ["-d", "opcache.enable=1", "-d", "opcache.enable_cli=1", *PHP_MODES[php_mode]]
    if choice == "none":
        return None
    cgi = shutil.which("php-cgi") if choice in ("auto", "fcgi") else None
    if cgi:
        port = free_port()
        return {
            "kind": "php-cgi-fastcgi",
            "label": "php-cgi",
            "detail": "the FastCGI SAPI FPM runs, opcache on",
            "argv": [cgi, "-b", f"127.0.0.1:{port}", *opcache],
            # The FastCGI SAPI exits after 500 requests unless told otherwise, which arrives at
            # the generator as a reset socket a third of the way into a run. FPM's own
            # `pm.max_requests` defaults to 0 for the same reason: recycling is a leak workaround,
            # and a benchmark that recycles is measuring process start-up.
            "env": {"PHP_FCGI_MAX_REQUESTS": "0"},
            "port": port,
            "executable": cgi,
            "version_argv": ["-v"],
            "open": lambda: FcgiConn(port, script),
        }
    if choice in ("auto", "builtin") and shutil.which(php):
        port = free_port()
        return {
            "kind": "php-builtin-server",
            "label": "php -S",
            "detail": "PHP's own development server, serial by construction",
            "argv": [php, *opcache, "-S", f"127.0.0.1:{port}", str(script)],
            "port": port,
            "executable": php,
            "version_argv": ["-v"],
            "open": lambda: HttpConn(port),
        }
    return None


def serve_vs_fpm(binary: Path, args, reps: int) -> int:
    """M7's throughput figure: `nvs serve` against PHP with opcache, both under one generator.

    `reps` is the resolved count rather than `args.reps`, which is a sentinel meaning the caller
    asked for nothing and each leg's own default stands.
    """
    entry = SERVE_DIR / f"{SERVE_CASE}.nvs"
    twin = SERVE_DIR / f"{SERVE_CASE}.php"
    if not entry.is_file():
        sys.exit(f"{entry} is missing; this leg's case is the twin pair under benches/serve")
    if args.requests < 1 or args.concurrency < 1:
        sys.exit("--requests and --concurrency must each be at least 1")

    port = free_port()
    novis = {
        "kind": "nvs-serve",
        "label": "nvs serve",
        "detail": "`rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s development server, one core",
        "argv": [str(binary), "serve", str(entry), "--listen", f"127.0.0.1:{port}"],
        "cwd": ROOT,
        "port": port,
        "executable": binary,
        "version_argv": ["--version"],
        "open": lambda: HttpConn(port),
    }
    peer = php_peer(args.php, args.php_mode, args.serve_baseline) if twin.is_file() else None

    if peer is not None and peer["kind"] == "php-cgi-fastcgi" and args.concurrency > 1:
        # Honoured where FPM would exist and ignored on Windows, where the SAPI has no fork to
        # make -- which is the whole of why `--concurrency` defaults to 1 here.
        peer["env"]["PHP_FCGI_CHILDREN"] = str(args.concurrency)

    caveats = []
    if peer is None and not twin.is_file():
        caveats.append(f"no PHP baseline: {twin.name} is missing")
    elif peer is None and args.serve_baseline == "none":
        caveats.append("no PHP baseline: --serve-baseline none, so this run is nvs serve alone")
    elif peer is None:
        caveats.append(
            f"no PHP baseline: --serve-baseline {args.serve_baseline} found nothing on PATH, "
            "so this run is nvs serve alone"
        )
    elif peer["kind"] == "php-builtin-server":
        caveats.append(
            "the baseline is `php -S`, PHP's development server -- a far weaker peer than the "
            "FastCGI SAPI FPM runs, and a ratio against it is not M7's figure"
        )
    else:
        caveats.append(
            "the baseline pays no HTTP parse and no proxy hop -- it is driven over FastCGI as "
            "nginx would drive FPM -- while nvs serve pays both; the comparison favours PHP"
        )
    if args.concurrency > 1:
        caveats.append(
            f"concurrency is {args.concurrency}: neither PHP peer here serves two requests at "
            "once, so anything above 1 is measuring Novis against a queue"
        )

    print(
        f"case {SERVE_CASE}: {args.requests} request(s) over {args.concurrency} keep-alive "
        f"connection(s), {reps} rep(s), best of reps"
    )
    measured = [measure_server(novis, args.requests, args.concurrency, reps)]
    if peer is not None:
        measured.append(measure_server(peer, args.requests, args.concurrency, reps))
    print()

    label_width = max(len(m["label"]) for m in measured)
    for result in measured:
        print(
            f"  {result['label']:<{label_width}}  {result['requests_per_sec']:>10,.0f} requests/sec"
            f"  {result['ms_per_request']:>8.3f} ms/request   {result['detail']}"
        )

    bodies = {result["label"]: result["body"] for result in measured}
    if len(set(bodies.values())) > 1:
        print()
        print("DIFF: the two servers did not answer with the same bytes, so neither number stands")
        for label, body in bodies.items():
            print(f"  {label} sent {body!r}")
        return 1

    for result in measured:
        if result["reconnects"]:
            caveats.append(
                f"{result['label']} declined to keep the connection open and paid "
                f"{result['reconnects']} extra handshake(s) inside the timed passes"
            )

    ratio = None
    if len(measured) == 2:
        ratio = measured[0]["requests_per_sec"] / measured[1]["requests_per_sec"]
        print(
            f"  nvs serve is {ratio:.2f}x {measured[1]['label']} at concurrency {args.concurrency}"
        )
    print()
    for caveat in caveats:
        print(f"  note: {caveat}")

    record = {
        "date": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "commit": version("git", ["rev-parse", "--short", "HEAD"]),
        "case": SERVE_CASE,
        "requests": args.requests,
        "concurrency": args.concurrency,
        "reps": reps,
        "php_mode": args.php_mode,
        "ratio": round(ratio, 4) if ratio is not None else None,
        "caveats": caveats,
        "nvs": _serve_entry(measured[0]),
        "baseline": _serve_entry(measured[1]) if len(measured) == 2 else None,
        "nvs_binary": str(binary.relative_to(ROOT)) if binary.is_relative_to(ROOT) else str(binary),
        "host": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "cpus": os.cpu_count(),
        },
    }
    if args.record:
        write_serve_record(Path(args.record), record)
    else:
        print("  not recorded: pass --record PATH to append this run to a JSON array")
    return 0


def _serve_entry(result: dict) -> dict:
    """One peer's half of the record: the numbers, never the body it sent."""
    return {
        "kind": result["kind"],
        "label": result["label"],
        "detail": result["detail"],
        "version": result["version"],
        "requests_per_sec": round(result["requests_per_sec"], 1),
        "median_requests_per_sec": round(result["median_requests_per_sec"], 1),
        "ms_per_request": round(result["ms_per_request"], 4),
        "reconnects": result["reconnects"],
    }


def write_serve_record(path: Path, record: dict) -> None:
    """Append one run to a JSON array, so the artifact stays a readable history.

    An array rather than the NDJSON `--json` writes: `benches/serve.json` is named by M7 with
    that extension, one run is one object rather than one object per case, and a file a human
    opens to read a headline number should parse as a whole.

    The history is capped at `SERVE_HISTORY` runs, oldest dropped, because the loop driver's
    acceptance check appends one every iteration -- an uncapped artifact is a diff nobody reads,
    and the run that matters is the most recent one on a given box. A figure worth keeping past
    that belongs in a doc that cites it.

    Where the caller points this is what makes the artifact. The loop's sweep records into the
    ignored `benches/results/`, because a row appended after a session's commits is a change no
    slice owns and nothing stages; `benches/serve.json` is tracked, so a row there is a figure
    published by hand. `docs/agent/commands.md` § *The server's throughput* is that split's home.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    history: list = []
    if path.is_file() and path.stat().st_size:
        try:
            history = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            sys.exit(f"{path} is not the JSON array this leg appends to ({exc}); move it aside")
        if not isinstance(history, list):
            sys.exit(f"{path} holds a {type(history).__name__}, not the JSON array this leg appends to")
    history = [*history, record][-SERVE_HISTORY:]
    path.write_text(json.dumps(history, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    shown = path.relative_to(ROOT) if path.is_absolute() and path.is_relative_to(ROOT) else path
    print(f"  recorded 1 run to {shown} ({len(history)} in the history)")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run the userland benchmark suite across Novis, PHP, Python and Bun side by side.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__.split("## The engine roster", 1)[0],
    )
    parser.add_argument("patterns", nargs="*", help="substrings; only matching cases run")
    parser.add_argument("--reps", type=int, default=None,
                        help=f"timed reps per engine (default {SUITE_REPS}, "
                             f"{WARM_START_REPS} on --warm-start)")
    parser.add_argument("--check", action="store_true", help="agreement only, no timing")
    parser.add_argument("--nvs", help="path to the nvs binary (default target/release)")
    parser.add_argument("--php", default="php", help="php executable (default `php`)")
    parser.add_argument(
        "--python",
        default=sys.executable,
        help="python executable (default: the one running this script)",
    )
    parser.add_argument("--bun", default="bun", help="bun executable (default `bun`)")
    parser.add_argument(
        "--engines",
        default="nvs,php,python,bun",
        help="comma-separated engine list, in table order; must include nvs (default all four)",
    )
    parser.add_argument(
        "--php-mode",
        choices=sorted(PHP_MODES),
        default="jit",
        help="`jit` (default) runs PHP with opcache+tracing JIT; `default` runs it as installed",
    )
    parser.add_argument("--allow-debug", action="store_true", help="permit a debug Novis binary")
    parser.add_argument(
        "--warm-start",
        action="store_true",
        help="measure the CLI's start floor on the baseline case instead of running the suite",
    )
    parser.add_argument(
        "--max-work-ms",
        type=float,
        metavar="MS",
        help="with --warm-start: exit non-zero if the CLI's own work -- the total less "
             "`nvs --version` -- exceeds this budget, in ms; a machine whose floor is itself "
             "multiples of it is reported as not measured",
    )
    # Retired, and loudly rather than silently: a `--max-ms 10` left in a goal file would go on
    # budgeting the whole wall clock, most of which is the OS. See `warm_start`.
    parser.add_argument("--max-ms", type=float, metavar="MS", help=argparse.SUPPRESS)
    parser.add_argument("--json", metavar="PATH", help="append one NDJSON record per case")
    parser.add_argument(
        "--serve-vs-fpm",
        action="store_true",
        help="measure `nvs serve` requests/sec against PHP with opcache, instead of the suite",
    )
    parser.add_argument(
        "--record",
        metavar="PATH",
        help="with --serve-vs-fpm: append the run to the JSON array at PATH",
    )
    parser.add_argument(
        "--requests",
        type=int,
        default=1000,
        metavar="N",
        help="with --serve-vs-fpm: requests per rep, per server (default 1000)",
    )
    parser.add_argument(
        "--concurrency",
        type=int,
        default=1,
        metavar="N",
        help="with --serve-vs-fpm: keep-alive connections (default 1; the module doc says why)",
    )
    parser.add_argument(
        "--serve-baseline",
        choices=("auto", "fcgi", "builtin", "none"),
        default="auto",
        help="with --serve-vs-fpm: which PHP peer -- `auto` takes php-cgi over php -S",
    )
    args = parser.parse_args()

    if args.max_ms is not None:
        sys.exit("--max-ms budgeted the whole wall clock, of which most is the operating system "
                 "creating a process rather than anything Novis does. Use --max-work-ms, which "
                 "budgets the total less `nvs --version`; `warm_start` in this file says why.")
    if args.reps is not None and args.reps < 1:
        sys.exit("--reps must be at least 1")
    asked_reps = args.reps if args.reps is not None else (
        WARM_START_REPS if args.warm_start else SUITE_REPS)
    reps = 1 if args.check else asked_reps

    binary = find_nvs(args.nvs, args.allow_debug)
    warn_if_stale(binary)
    if args.warm_start:
        # Before the roster, because a start figure is one engine's and must not need PHP or Bun
        # installed to be measured. `asked_reps` rather than `reps`: `--check` narrows the suite to
        # agreement, and there is nothing here to agree with.
        return warm_start(binary, asked_reps, args.max_work_ms)
    if args.serve_vs_fpm:
        # Before the roster, for `--warm-start`'s reason: this leg is one server against one
        # PHP peer, and must not need Python or Bun installed to produce its number.
        # `asked_reps`, as `--warm-start` above: `--check` has nothing to agree with here either.
        return serve_vs_fpm(binary, args, asked_reps)
    if args.record:
        sys.exit("--record writes the serve leg's artifact, and needs --serve-vs-fpm")
    if args.max_work_ms is not None:
        sys.exit("--max-work-ms is a budget on the warm-start figure, and needs --warm-start")
    selected = [name.strip() for name in args.engines.split(",") if name.strip()]
    engines = build_engines(selected, binary, args.php, args.php_mode, args.python, args.bun)
    cases = discover(args.patterns, engines)
    if not cases:
        sys.exit("no runnable case found under benches/userland")

    for engine in engines:
        target = binary if engine.key == PRIMARY else engine.argv[0]
        mode = f" [{args.php_mode}]" if engine.key == "php" else ""
        print(f"{engine.label:<4} {target}{mode}  ({version(target, engine.version_argv)})")
    print(
        f"{len(cases)} case(s), {reps} rep(s), min of reps;"
        f" a ratio above 1.00 means Novis is faster"
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

    The `nvs_ms`/`php_ms`/`ratio` keys are kept at the top level unchanged so the history file
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
                "nvs_ms": per_engine.get("nvs", {}).get("ms"),
                "php_ms": per_engine.get("php", {}).get("ms"),
                "nvs_work_ms": per_engine.get("nvs", {}).get("work_ms"),
                "php_work_ms": per_engine.get("php", {}).get("work_ms"),
                "ratio": per_engine.get("php", {}).get("ratio"),
                "engines": per_engine,
                "engine_versions": versions,
                "reps": reps,
                "php_mode": mode,
                "nvs_binary": str(binary.relative_to(ROOT)) if binary.is_relative_to(ROOT) else str(binary),
                "host": host,
            }, ensure_ascii=False) + "\n")
            written += 1
    print(f"appended {written} record(s) to {path}")


if __name__ == "__main__":
    sys.exit(main())
