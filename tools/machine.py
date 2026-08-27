#!/usr/bin/env python3
"""How wide this machine may run something, probed once and remembered.

    python tools/machine.py             # what this box is, and the widths it implies
    python tools/machine.py --refresh   # probe again, forgetting what was cached
    python tools/machine.py --json      # the cache as it stands, for a tool to read

## Why this exists

The valgrind sweep used to run four fixtures at a time because four was measured to be right *on
one box* -- a 16-core Windows machine, through WSL. The number was correct and the reasoning was
sound; what it could not do is travel. On an 8-core laptop four workers is most of the machine, and
on a 32-core one it leaves the sweep at a third of what the box could give.

So the constant is gone and this file holds the policy instead. What a machine *is* -- how many
cores the work will actually see, how much memory is free where it runs, what one unit of that work
costs serially -- is measured once and cached in `.loop/machine.json`, because none of it changes
between runs and probing it is the only part with a cost.

## The policy, and its one home

`width()` is the only place a parallel width is decided, for every caller. **Half the cores the
work will actually see**, never fewer than two, never more than there are items to run, and never
more than free memory divided by what one worker was measured to hold.

Half, and not more, because the machine is not idle. `tools/loop.py` starts a background release
build before the valgrind sweep and lets the two overlap deliberately -- that build is the longest
pole of an acceptance check, and cores handed to the sweep come straight out of it. The sweep's own
scaling flattens well before the box is full: measured over 23 fixtures on the 16-core box, 67.8s
at 1, 21.2s at 4 (3.2x), 16.0s at 8 (4.2x). Half of 16 buys the 4.2x; three quarters would buy
perhaps a tenth more of a check the build already dominates.

The floor of two is for small boxes: a 4-core runner still halves its sweep, and only a genuinely
single-core machine runs anything serially.

## What is cached, and when it is not

One entry per *execution context* -- `wsl` and `native` are different machines as far as this is
concerned, and on Windows they are different core counts (WSL2 takes its CPU and memory limits from
`.wslconfig`, not from the host). An entry is re-probed when the host name changes, when the format
version moves, or after `STALE_DAYS`, which is the cheap way to notice a `.wslconfig` edit.

`NVS_JOBS` overrides every width; `NVS_VALGRIND_JOBS` overrides the sweep's alone. Both are for a
one-off -- neither is written back to the cache.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

#: Beside `.loop/goal-green.json`, for the same reason: gitignored, per-clone, and never pruned --
#: `tools/disk.py` retires `.loop/logs` and nothing else under it.
CACHE = ROOT / ".loop" / "machine.json"

#: The cache format. Bumped when a probe learns a new field the width depends on.
VERSION = 1

#: The policy, in three numbers. See the module docstring for why it is half and not more.
FRACTION = 0.5
FLOOR = 2
STALE_DAYS = 30

#: What a probe reports, and the only keys read back out of the cache.
NUMERIC = ("cores", "mem_kb", "sample_s", "sample_rss_kb", "sample_code")


# ------------------------------------------------------------------------------ the policy


def width(cores, *, ceiling=None, mem_kb=None, worker_kb=None,
          fraction=FRACTION, floor=FLOOR):
    """How many of `cores` this may use, under every cap that applies.

    `ceiling` is how many items there are -- more workers than work is only process launches.
    `mem_kb` with `worker_kb` caps on memory: half of what was free when the box was probed,
    divided by what one worker was measured to hold. Either missing means no memory cap, which is
    the right default rather than a guessed one: at half the cores it has never been the binding
    constraint on a machine that could run the work at all.
    """
    n = max(1, int(cores or 1))
    want = max(floor if n > 1 else 1, int(n * fraction))
    if ceiling:
        want = min(want, max(1, int(ceiling)))
    if mem_kb and worker_kb:
        want = min(want, max(1, int(mem_kb) // 2 // max(1, int(worker_kb))))
    return max(1, min(want, n))


def override(*names):
    """The first of `names` set in the environment to a positive integer, or None."""
    for name in names:
        raw = os.environ.get(name, "").strip()
        if raw.isdigit() and int(raw) > 0:
            return int(raw)
    return None


# ------------------------------------------------------------------------------- probing


#: One shell line, run where the work runs. No double quotes and no newlines anywhere in it: on
#: Windows this reaches a Linux shell as one argv element of `wsl.exe`, and the fewer layers that
#: can reinterpret it the better. Every part degrades to silence rather than to an error, so a
#: probe on a box without `/proc` or without `/usr/bin/time` simply reports less.
PROBE_SH = (
    "echo cores $(nproc 2>/dev/null || echo 0); "
    "grep MemAvailable /proc/meminfo 2>/dev/null | sed s/MemAvailable:/mem_kb/"
)

#: The serial baseline, appended to `PROBE_SH` when the caller hands over one unit of the real
#: work. It answers two questions in the one call the probe already costs: what a single item costs
#: with nothing else running -- which is what makes a later sweep's speedup a measurement rather
#: than an estimate -- and what one worker holds, which is where the memory cap comes from.
SAMPLE_SH = (
    "; if command -v /usr/bin/time >/dev/null 2>&1; then "
    "/usr/bin/time -o /tmp/nvs-machine-probe -f 'sample_s %e sample_rss_kb %M' "
    "sh -c '{sample}' >/dev/null 2>&1; echo sample_code $?; "
    "cat /tmp/nvs-machine-probe 2>/dev/null; rm -f /tmp/nvs-machine-probe; fi"
)


def probe_sh(sample=None):
    """The whole probe as one shell line -- facts, and a timed sample when there is one."""
    if not sample:
        return PROBE_SH
    if "'" in sample:  # `sh -c '...'` cannot carry one, and no caller has ever needed to
        return PROBE_SH
    return PROBE_SH + SAMPLE_SH.format(sample=sample)


def read_probe(text):
    """`{field: number}` out of whatever the probe managed to print.

    Deliberately forgiving: every field is optional, `mem_kb 15000 kB` and `cores 16` and
    `sample_s 3.21 sample_rss_kb 55120` all read the same way, and anything unrecognised is
    ignored rather than being an error. A probe that prints nothing is a machine with one core as
    far as `width()` is concerned, which is the safe direction to be wrong in.
    """
    found = {}
    for key, value in re.findall(r"\b([a-z_]+)\s+(\d+(?:\.\d+)?)", text or ""):
        if key in NUMERIC and key not in found:
            found[key] = float(value) if "." in value else int(value)
    return found


def posix_probe(runner, sample=None):
    """Run the probe through `runner(shell_line) -> (code, text)` and read what came back.

    `runner` is how the caller reaches the context being probed: `wsl.exe -- bash -lc` for the WSL
    leg, a local `bash -lc` for a native Linux one.
    """
    code, text = runner(probe_sh(sample))
    got = read_probe(text)
    if code != 0 and not got:
        return {}
    if got.pop("sample_code", 0) != 0:
        # The sample ran and failed. Its timing says nothing about the cost of the work, and the
        # failure itself is the caller's business -- the sweep will hit the same fixture. Mark the
        # entry so the next run measures again: a red tree is temporary, and an entry that will
        # never hold a baseline is not what a failed run should leave behind.
        for key in ("sample_s", "sample_rss_kb"):
            got.pop(key, None)
        got["sample_pending"] = 1
    # No `sample_code` at all means the box has no `/usr/bin/time`, which will still be true
    # tomorrow. That entry is complete as it stands: the width never needed a baseline, only the
    # memory cap and the reporting did.
    return got


def local_probe():
    """This interpreter's own machine, without leaving the process where it can be helped.

    No sample: the only caller that runs here is `tools/try.py`, whose unit of work is a snippet
    that takes milliseconds and holds nothing. A baseline is worth a probe's cost for valgrind and
    not for that.
    """
    got = {"cores": os.cpu_count() or 1}
    try:
        for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("MemAvailable:"):
                got["mem_kb"] = int(line.split()[1])
                break
    except (OSError, ValueError, IndexError):
        pass
    return got


def _bash(line):
    try:
        p = subprocess.run(["bash", "-lc", line], capture_output=True,
                           encoding="utf-8", errors="replace", timeout=300)
    except (OSError, subprocess.SubprocessError):
        return 1, ""
    return p.returncode, (p.stdout or "") + (p.stderr or "")


# --------------------------------------------------------------------------------- cache


def _today():
    return _dt.date.today().isoformat()


def _load():
    try:
        doc = json.loads(CACHE.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    return doc if isinstance(doc, dict) and doc.get("version") == VERSION else {}


def _save(doc):
    doc["version"] = VERSION
    try:
        CACHE.parent.mkdir(exist_ok=True)
        CACHE.write_text(json.dumps(doc, indent=1, sort_keys=True),
                         encoding="utf-8", newline="\n")
    except OSError:
        pass  # a machine profile is an optimisation; failing to keep it is not a failure


def stale(entry, days=STALE_DAYS):
    """Whether `entry` must be probed again. A different host is the obvious case; the age is for
    the one that is not observable -- a `.wslconfig` edit changes how many cores a leg sees and
    leaves no trace this side of the probe."""
    if not isinstance(entry, dict) or not entry.get("cores"):
        return True
    if entry.get("host") != platform.node():
        return True
    if entry.get("sample_pending"):
        return True  # the baseline could not be taken last time; try once more
    try:
        probed = _dt.date.fromisoformat(entry.get("probed", ""))
    except ValueError:
        return True
    return (_dt.date.today() - probed).days > days


def profile(context, probe=None, refresh=False):
    """The cached facts for one execution context, probing when there is nothing usable.

    `context` is where the work runs -- `wsl`, `native`, `local` -- and not which caller is asking:
    two tools running work in the same place are looking at the same machine.
    """
    doc = _load()
    entries = doc.setdefault("contexts", {})
    entry = entries.get(context)
    if not refresh and not stale(entry):
        return entry
    if probe is None and context != "local":
        # Nobody can reach that context from here -- only the tool that runs work there can probe
        # it. Whatever it left behind is better than a guess.
        return entry if isinstance(entry, dict) and entry.get("cores") else {"cores": 1}
    got = (probe or local_probe)()
    if not got.get("cores"):
        # Nothing usable came back. Keep whatever was cached rather than replacing it with a
        # blank -- a WSL that was busy for one probe is still the same machine it was.
        return entry if isinstance(entry, dict) and entry.get("cores") else {"cores": 1}
    entry = {"host": platform.node(), "probed": _today(), **got}
    entries[context] = entry
    _save(doc)
    return entry


def remember(context, **fields):
    """Fold measured numbers into a context's entry -- a sweep's real speedup, say -- without
    re-probing anything. Silently does nothing when the context has never been probed."""
    doc = _load()
    entry = doc.get("contexts", {}).get(context)
    if not isinstance(entry, dict):
        return
    entry.update({k: v for k, v in fields.items() if v is not None})
    _save(doc)


def jobs(context, *, ceiling=None, probe=None, envs=(), refresh=False):
    """The width for work in `context`: an override if one is set, the policy otherwise."""
    forced = override(*envs, "NVS_JOBS")
    if forced:
        return max(1, min(forced, ceiling)) if ceiling else forced
    p = profile(context, probe=probe, refresh=refresh)
    return width(p.get("cores"), ceiling=ceiling,
                 mem_kb=p.get("mem_kb"), worker_kb=p.get("sample_rss_kb"))


# ----------------------------------------------------------------------------------- cli


def show(doc, as_json):
    if as_json:
        print(json.dumps(doc, indent=1, sort_keys=True))
        return 0
    contexts = doc.get("contexts", {})
    if not contexts:
        print("machine: nothing probed yet. The first valgrind sweep or `try.py` run fills this in,")
        print("         or `python tools/machine.py --refresh` does it now.")
        return 0
    print(f"{'context':<9} {'cores':>5} {'free':>8} {'serial':>8} {'per worker':>11}  "
          f"{'width':>5}  probed")
    for name, e in sorted(contexts.items()):
        mem = f"{e['mem_kb'] / 1048576:.1f}G" if e.get("mem_kb") else "--"
        secs = f"{e['sample_s']:.1f}s" if e.get("sample_s") else "--"
        rss = f"{e['sample_rss_kb'] / 1024:.0f}M" if e.get("sample_rss_kb") else "--"
        w = width(e.get("cores"), mem_kb=e.get("mem_kb"), worker_kb=e.get("sample_rss_kb"))
        stamp = e.get("probed", "?") + (" (stale)" if stale(e) else "")
        print(f"{name:<9} {e.get('cores', 0):>5} {mem:>8} {secs:>8} {rss:>11}  {w:>5}  {stamp}")
    print(f"\n  {int(FRACTION * 100)}% of the cores the work sees, floor {FLOOR}, capped by the "
          f"item count and by\n  free memory. NVS_JOBS overrides every width for one run; "
          f"NVS_VALGRIND_JOBS the sweep's.")
    return 0


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--refresh", action="store_true",
                    help="re-probe every cached context now, instead of when it goes stale")
    ap.add_argument("--json", action="store_true", help="print the cache as JSON")
    opts = ap.parse_args()

    if opts.refresh:
        doc = _load()
        # `local` is probed from here. Every other context is reachable only by the tool that runs
        # work there, so refreshing one means dropping it and letting that tool probe on its next
        # run -- which is when the machine is set up for the probe anyway.
        dropped = [n for n in sorted(doc.get("contexts", {})) if n != "local"]
        for name in dropped:
            doc["contexts"].pop(name)
        if dropped:
            _save(doc)
            print(f"machine: dropped {', '.join(dropped)} -- the next run that uses one re-probes it")
        profile("local", refresh=True)
    else:
        profile("local")
    return show(_load(), opts.json)


if __name__ == "__main__":
    sys.exit(main())
