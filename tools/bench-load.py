#!/usr/bin/env python3
"""The load leg: what `nvs serve` saturates at on this machine, with every answer verified.

    python tools/bench-load.py                          # the sweep, then the in-flight leg
    python tools/bench-load.py --record benches/serve-load.json   # append the run to the artifact
    python tools/bench-load.py --concurrency 1,16,256   # drive exactly these widths
    python tools/bench-load.py --seconds 30             # longer points, for a figure worth keeping
    python tools/bench-load.py --in-flight 0            # the sweep alone
    python tools/bench-load.py --probe path/to/nvs-serve-probe   # skip the cargo build

`benches/serve-probe/` is the generator and its module doc owns the client half -- the token every
request carries, the two modes, and what a run does not measure. This file owns only how a run is
carried out on whatever machine it is sitting on.

## What this leg is, and why it is a third one

The three serve legs answer three different questions and no arithmetic crosses between them:

* `tools/bench.py --serve-vs-fpm` compares against PHP on this box with no proxy. It is goal
  `server`'s acceptance check, it must keep working where there is no Docker and no `wrk`, and its
  generator is written into that file so it needs nothing built.
* `tools/bench-proxied.py` puts nginx in front of both peers in containers, which is the only
  deployment either has.
* **This one has no peer at all.** It asks what this machine's server saturates at, and -- the part
  neither other leg can answer -- whether every response was the answer to the request that asked
  for it. Both of those need a generator faster than the server, which is why this leg is the one
  with something to build.

**A throughput figure over one constant response is not evidence of a correct server.** Both other
legs drive `hello.nvs`, whose every answer is byte-identical, so a response delivered to the wrong
connection is indistinguishable from the right one and no generator can see it. This leg drives
`benches/serve/echo.nvs`, whose body is the token the request carried, and the probe rejects an
answer that names a different request. `nvs-serve-probe`'s own
`a_constant_body_fails_every_request` is what holds the check honest: point the verifier at the
constant case and every request must fail.

## Why the sweep is derived from the core count and not written down

A fixed concurrency list measures a different thing on every machine -- 64 connections is past
saturation on four cores and short of it on sixty-four. So the default straddles *this* box:
one connection, one per core, and multiples of the core count out to where the queue is plainly
the binding constraint. `--concurrency` overrides it, and the host's core count is in the record,
because a row whose machine is not written beside it measures nothing.

## The client is what runs out first, and a run says so

A connection costs the client an ephemeral port and a descriptor, and both are exhausted long
before a server is. Two things follow, and both are here rather than in the probe:

* The descriptor limit is raised to its hard ceiling before anything is started, on the platforms
  that have one. The probe inherits it.
* **The sweep runs before the in-flight leg, never after.** The in-flight leg opens ten thousand
  sockets and leaves them in `TIME_WAIT` for minutes; a sweep behind it would be measuring a client
  that cannot open connections and reporting it as the server slowing down. `--settle` is the wait
  between the two for a run that wants to be sure.

A run that could not open what it asked for reports `client_limited` and the operating system's own
error, so a client ceiling is never recorded as the server's.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent

# `tools/bench.py` holds the nvs-binary lookup, the free-port probe, the server subprocess with its
# drained pipe, and the record writer. Imported rather than reimplemented, for the reason
# `bench-proxied.py` gives at the same import: a second copy is a second thing to keep correct.
sys.path.insert(0, str(HERE))
import bench  # noqa: E402  -- same directory; these helpers have one home and it is there

CASE = ROOT / "benches" / "serve" / "echo.nvs"
PROBE_PACKAGE = "nvs-serve-probe"
PROBE_BIN = "nvs-serve-probe"


def probe_path(explicit: str | None) -> Path:
    """The probe binary, built if it is not already there.

    Built in release: a debug generator is slower than the server it is driving, which makes every
    number the generator's own speed rather than the server's.
    """
    if explicit:
        path = Path(explicit)
        if not path.is_file():
            sys.exit(f"--probe {path} is not a file")
        return path
    suffix = ".exe" if os.name == "nt" else ""
    built = ROOT / "target" / "release" / f"{PROBE_BIN}{suffix}"
    print(f"  building {PROBE_PACKAGE} (release)")
    done = subprocess.run(
        ["cargo", "build", "--release", "-p", PROBE_PACKAGE],
        cwd=str(ROOT),
        check=False,
    )
    if done.returncode != 0:
        sys.exit(f"cargo build -p {PROBE_PACKAGE} failed with {done.returncode}")
    if not built.is_file():
        sys.exit(f"{PROBE_PACKAGE} built but {built} is not there")
    return built


def raise_descriptor_limit() -> str | None:
    """Lift the soft descriptor limit to the hard one, and say what it now is.

    macOS ships a soft limit of 256 and Linux 1024, either of which stops the in-flight leg long
    before the server does -- and the failure looks exactly like a server that will not accept.
    Windows has no such limit and answers `None`.
    """
    try:
        import resource  # noqa: PLC0415  -- POSIX only, and this is the only caller
    except ImportError:
        return None
    soft, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
    if soft < hard:
        try:
            resource.setrlimit(resource.RLIMIT_NOFILE, (hard, hard))
            soft = hard
        except (ValueError, OSError):
            pass
    return f"{soft}"


def default_sweep(cores: int) -> list[int]:
    """One connection, one per core, and out past saturation in multiples of the core count."""
    widths = {1, 2, cores, cores * 2, cores * 8, cores * 32}
    return sorted(width for width in widths if 0 < width <= 2048)


def run_probe(probe: Path, port: int, mode: str, connections: int, **flags: object) -> dict:
    """One probe run, as its record. A probe that could not parse its own arguments is fatal."""
    argv = [
        str(probe),
        "--target", f"127.0.0.1:{port}",
        "--mode", mode,
        "--connections", str(connections),
    ]
    for name, value in flags.items():
        argv += [f"--{name.replace('_', '-')}", str(value)]
    done = subprocess.run(argv, capture_output=True, text=True, check=False)
    if done.returncode == 2 or not done.stdout.strip():
        sys.exit(f"{PROBE_BIN} refused the run: {done.stderr.strip() or done.returncode}")
    try:
        record = json.loads(done.stdout)
    except json.JSONDecodeError as exc:
        sys.exit(f"{PROBE_BIN} did not print a record ({exc}):\n{done.stdout}\n{done.stderr}")
    record["wrong"] = done.returncode == 1
    return record


def sweep_table(rows: list[dict]) -> None:
    """One line per concurrency, with the verification columns beside the throughput ones."""
    print()
    print(f"  {'conns':>6}  {'req/s':>10}  {'p50 us':>8}  {'p99 us':>8}  {'max us':>10}  "
          f"{'verified':>10}  {'wrong':>6}  {'stalls':>7}  note")
    for row in rows:
        wrong = row["bad_status"] + row["bad_body"] + row["leftover"]
        note = "" if not row["client_limited"] else f"client-limited at {row['connections_opened']}"
        print(
            f"  {row['connections_requested']:>6}  {row['requests_per_sec']:>10,.0f}  "
            f"{row['p50_us']:>8,}  {row['p99_us']:>8,}  {row['max_us']:>10,.0f}  "
            f"{row['verified']:>10,}  {wrong:>6,}  {row['stalls']:>7,}  {note}"
        )


def in_flight_table(record: dict) -> None:
    """One line per round of the simultaneity leg."""
    print()
    opened = record["connections_opened"]
    asked = record["connections_requested"]
    if record["client_limited"]:
        print(f"  in flight: opened {opened:,} of {asked:,} -- {record['connect_error']}")
        print("             the ceiling below is this client's, not the server's")
    for row in record["rounds"]:
        print(
            f"  round {row['round']}: in_flight={row['in_flight']:,} "
            f"verified={row['verified']:,} mismatched={row['mismatched']:,} "
            f"broken={row['broken']:,} unanswered={row['unanswered']:,} "
            f"all_answered_in={row['all_answered_in_s']:.3f}s"
        )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Saturate `nvs serve` on this machine and verify every answer.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__.split("## What this leg is", 1)[0],
    )
    parser.add_argument("--nvs", help="path to the nvs binary (default target/release)")
    parser.add_argument("--allow-debug", action="store_true", help="permit a debug Novis binary")
    parser.add_argument("--probe", help="path to a built nvs-serve-probe (default: build it)")
    parser.add_argument(
        "--concurrency",
        help="comma-separated connection counts for the sweep (default: derived from the core count)",
    )
    parser.add_argument("--seconds", type=int, default=10, help="seconds per sweep point (default 10)")
    parser.add_argument(
        "--in-flight",
        type=int,
        default=10_000,
        dest="in_flight",
        help="requests to put in flight at once, or 0 to skip that leg (default 10000)",
    )
    parser.add_argument("--rounds", type=int, default=3, help="in-flight rounds (default 3)")
    parser.add_argument(
        "--settle",
        type=int,
        default=0,
        help="seconds to wait between the sweep and the in-flight leg, for ports to drain",
    )
    parser.add_argument("--timeout-ms", type=int, default=5_000,
                        help="a response slower than this is a stall (default 5000)")
    parser.add_argument("--record", help="append this run to the JSON array at PATH")
    args = parser.parse_args()

    if not CASE.is_file():
        sys.exit(f"{CASE} is missing; this leg's case is part of the tree")
    binary = bench.find_nvs(args.nvs, args.allow_debug)
    bench.warn_if_stale(binary)
    probe = probe_path(args.probe)
    descriptors = raise_descriptor_limit()
    cores = os.cpu_count() or 1
    widths = (
        [int(width) for width in args.concurrency.split(",") if width.strip()]
        if args.concurrency
        else default_sweep(cores)
    )

    port = bench.free_port()
    print(f"  serving {CASE.relative_to(ROOT)} on 127.0.0.1:{port} ({cores} cores"
          + (f", {descriptors} descriptors)" if descriptors else ")"))
    server = bench.Server(
        [str(binary), "serve", CASE.name, "--port", str(port)],
        port,
        cwd=CASE.parent,
    )
    rows: list[dict] = []
    flight: dict | None = None
    try:
        for width in widths:
            print(f"  closed loop: {width} connection(s) for {args.seconds}s")
            rows.append(run_probe(
                probe, port, "closed-loop", width,
                seconds=args.seconds, timeout_ms=args.timeout_ms,
            ))
        if args.in_flight > 0:
            if args.settle:
                print(f"  settling {args.settle}s before the in-flight leg")
                time.sleep(args.settle)
            print(f"  in flight: {args.in_flight} at once, {args.rounds} round(s)")
            flight = run_probe(
                probe, port, "in-flight", args.in_flight,
                rounds=args.rounds, timeout_ms=args.timeout_ms,
            )
    finally:
        server.stop()

    sweep_table(rows)
    if flight:
        in_flight_table(flight)

    wrong = sum(row["bad_status"] + row["bad_body"] + row["leftover"] for row in rows)
    if flight:
        wrong += sum(
            row["mismatched"] + row["broken"] + row["unanswered"] for row in flight["rounds"]
        )
    notes = [note for row in rows for note in row["notes"]]
    notes += flight["notes"] if flight else []
    print()
    if wrong:
        print(f"  FAIL: {wrong:,} answer(s) did not belong to the request that asked")
        for note in notes[:10]:
            print(f"    ! {note}")
    else:
        best = max((row["requests_per_sec"] for row in rows), default=0.0)
        served = sum(row["verified"] for row in rows)
        print(f"  every one of {served:,} answers belonged to the request that asked")
        print(f"  peak {best:,.0f} req/s across {len(rows)} concurrency point(s)")

    record = {
        "date": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "commit": bench.version("git", ["rev-parse", "--short", "HEAD"]),
        "case": CASE.name,
        "seconds_per_point": args.seconds,
        "nvs_version": bench.version(str(binary), ["--version"]),
        "nvs_binary": str(binary.relative_to(ROOT)) if binary.is_relative_to(ROOT) else str(binary),
        "sweep": rows,
        "in_flight": flight,
        "wrong": wrong,
        "host": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "cpus": cores,
            "descriptor_limit": descriptors,
        },
    }
    if args.record:
        bench.write_serve_record(Path(args.record), record)
    else:
        print("  not recorded: pass --record PATH to append this run to a JSON array")
    return 1 if wrong else 0


if __name__ == "__main__":
    raise SystemExit(main())
