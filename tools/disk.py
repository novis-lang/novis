#!/usr/bin/env python3
"""What this tree costs on disk, and the one command that reclaims it.

    python tools/disk.py               # what is on disk, what is reclaimable, what is free
    python tools/disk.py --clean       # sweep it
    python tools/disk.py --clean -n    # say what --clean would delete; delete nothing
    python tools/disk.py --deep        # also size the four places outside this repository

Nothing here runs on the session path. `tools/loop.py` runs `clean` -- the whole of `--clean` --
after every session's acceptance check: between sessions, which is the one moment the driver
knows nothing is building and the build is warm. A person runs the same `--clean` by hand, and it
refuses while a driver holds `.loop/running`, because a person cannot see whether a session is
mid-build.

Why this exists. Cargo never garbage-collects `target/`: every dependency bump, feature change
or toolchain bump leaves the previous crate hash's rlib, rmeta, PDB and incremental directory
behind forever, and nothing on stable removes them -- `cargo clean gc` is still nightly-only as
of 1.97. `.loop/logs` and `.agent-tmp` are append-only for the same reason: nothing was ever
told to delete from them. An unattended run that fills the disk dies mid-session with the tree
half-edited, which is the worst possible moment for it, so the driver checks free space at the
door instead (`free_gb` below, `MIN_FREE_GB`).

What fills `target/` is *generations*. A crate's artifacts are named `<name>-<metadata-hash>`,
and that hash covers the dependency graph -- so every `Cargo.toml` or `Cargo.lock` edit mints a
fresh set for every crate downstream of the change and orphans the previous one. Editing source
does not: a source-only rebuild reuses every hash and adds nothing. Measured here on
2026-08-25, one generation of this workspace is ~6 GB and nine of them were on disk at once,
because the loop had added a dependency in most of its recent sessions.

Which is why age alone never condemns anything in `deps/`, the way `cargo-sweep` would have it.
Cargo never rewrites an artifact it has decided is still fresh, so a superseded generation and a
live one can carry the same date -- both were last written by the build that needed them. The
live set is *asked for* instead: the cargo commands in `LIVE_QUERIES`, each warm and with
`--message-format=json`, name every file the graph `tools/verify.py` builds actually uses.

Age is the second key, and only ever as a reason to *keep*. Every debug build is one of the shapes
in `LIVE_QUERIES` -- AGENTS.md's rule against `-p` is what makes that true, and commands.md § *A
debug cargo command never takes `-p`* is the measurement -- so the live set names everything a
session's builds use, and the grace only has to cover a build a session is still in the middle
of and the one target flag the rule leaves open, a `--test <name>` off the test family, which
resolves one dev-dependency its own way. Nothing written in the last `GRACE_HOURS` is swept
whatever the list says, and a variant idle for longer costs one rebuild the next time something
asks for it. The grace was a day once: a `-p` run per session then minted a copy of every
workspace crate, every copy was younger than that, and the sweep freed nothing. `incremental/`
is a cache with no artifact list to ask, so there age and the newest `KEEP_INCREMENTAL` per
crate are the whole rule. `release/deps` is never swept: its live set can only be asked for with
a release build, the slowest build here.

Nothing here can produce a wrong build. Cargo re-checks every fingerprint against the files
that are really on disk, so the worst a mistake costs is rebuilding something that was still
wanted.

Everything outside this repository is *reported* and never touched. The two WSL target
directories, the harness's transcripts and the cargo registry are other tools' state, and a repo
script that silently deletes another tool's state is a bug however much space it frees.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

TARGET = ROOT / "target"
LOGDIR = ROOT / ".loop" / "logs"
SCRATCH = ROOT / ".agent-tmp"
RUNNING = ROOT / ".loop" / "running"

# The retention policy has exactly one home, and this is it. `tools/loop.py` imports these
# rather than restating them.
KEEP_RUNS = 5  # .loop/logs: how many loop runs keep their session logs
SCRATCH_DAYS = 2  # .agent-tmp: older than this belongs to no session that is still running
KEEP_INCREMENTAL = 2  # target/*/incremental: cache generations kept per crate
MIN_FREE_GB = 10  # below this, tools/loop.py will not start a run
GRACE_HOURS = 2  # target/: nothing written more recently than this is swept, whatever cargo says

# `libnvs_stdlib-2a3f7eaa9f84477e.rlib` -> `libnvs_stdlib`. Cargo's metadata hash is 16 hex
# digits and always the last dash-separated component of the stem.
HASHED = re.compile(r"^(.+)-[0-9a-f]{16}$")

# Reported, never deleted. `{home}` is expanded against the user's home directory.
ELSEWHERE = [
    (
        "WSL Linux target",
        "{tmp}/nvs-linux",
        "the valgrind leg's own target dir, inside the ext4 vhdx",
        "wsl.exe -- rm -rf /var/tmp/nvs-linux    # frees ext4 space; see commands.md for the vhdx",
    ),
    (
        "WSL loop target",
        "{tmp}/nvs-target-wsl",
        "the loop's WSL leg target dir, beside it and just as large",
        "wsl.exe -- rm -rf /var/tmp/nvs-target-wsl    # the next leg pays a 32s cold build",
    ),
    (
        "harness transcripts",
        "{home}/.claude/projects",
        "one JSONL per session, kept forever by the harness",
        "delete the oldest project directories by hand",
    ),
    (
        "cargo registry",
        "{home}/.cargo/registry",
        "downloaded crate sources and their extracted copies",
        "rm -rf ~/.cargo/registry/src    # cargo re-extracts on the next build",
    ),
]


# ------------------------------------------------------------------------------ measuring


def human(n):
    for unit in ("B", "K", "M", "G", "T"):
        if n < 1024 or unit == "T":
            return f"{n:.0f}{unit}" if unit == "B" else f"{n:.1f}{unit}"
        n /= 1024
    return f"{n:.1f}T"


def walk(root):
    """(total bytes, [(path, size, mtime)]) for every file under `root`. Missing dir -> zeroes.

    One walk, because every caller wants both the total and the per-file ages, and `target/`
    is ~37k files: doing it twice is the difference between one second and two."""
    total = 0
    files = []
    if not root.is_dir():
        return 0, files
    for dirpath, _dirnames, filenames in os.walk(root):
        for name in filenames:
            p = Path(dirpath) / name
            try:
                st = p.stat()
            except OSError:
                continue  # vanished under us, or a link we cannot follow -- not our problem
            total += st.st_size
            files.append((p, st.st_size, st.st_mtime))
    return total, files


def free_gb(path=ROOT):
    """Free space, in GiB, on the volume holding `path`. 0.0 if it cannot be asked."""
    try:
        return shutil.disk_usage(path).free / 1024**3
    except OSError:
        return 0.0


def older_than(files, days):
    cutoff = time.time() - days * 86400
    return [(p, size) for p, size, mtime in files if mtime < cutoff]


# ------------------------------------------------------------------------------- deleting


def rm(path, dry_run):
    """Delete a file or a directory tree. Returns the bytes freed, 0 on any failure.

    Never raises: a cleanup pass that aborts halfway because one file was locked has done the
    worst of both things -- deleted some of what it meant to, and reported nothing."""
    try:
        if path.is_dir():
            size = walk(path)[0]
            if not dry_run:
                shutil.rmtree(path, ignore_errors=True)
            return size
        size = path.stat().st_size
        if not dry_run:
            path.unlink()
        return size
    except OSError:
        return 0


def run_id_of(name):
    """`20260825-114204-0001.log` -> `20260825-114204`. Everything else -> ''.

    The run stamp, not the session index: `run_session` restarts the index at 1 every run, so
    grouping by anything else would mix last week's session 3 into this week's."""
    parts = name.split("-")
    if len(parts) >= 3 and parts[0].isdigit() and parts[1].isdigit():
        return f"{parts[0]}-{parts[1]}"
    return ""


def prune_logs(keep=KEEP_RUNS, dry_run=False):
    """Keep the newest `keep` runs' logs under .loop/logs; delete the rest. Returns bytes freed.

    Whole runs rather than newest-N-files, because loop-stats.py reads a run as a unit and half
    a run measures nothing. Called once per run by the driver: one listdir of ~30 entries."""
    if not LOGDIR.is_dir():
        return 0
    entries = sorted(LOGDIR.iterdir(), key=lambda p: p.name)
    runs = sorted({run_id_of(p.name) for p in entries} - {""})
    doomed = set(runs[:-keep]) if keep > 0 else set(runs)
    return sum(rm(p, dry_run) for p in entries if run_id_of(p.name) in doomed)


def prune_scratch(days=SCRATCH_DAYS, dry_run=False):
    """Delete .agent-tmp entries nothing has written for `days`. Returns bytes freed.

    By age and not wholesale: the driver sweeps between sessions, but a person may be reading a
    verify log the session that just exited wrote, and that log is minutes old, not days. A
    directory is as old as its newest file (`newest`), so a `proof/` run still being written is
    kept for as long as anything in it is."""
    if not SCRATCH.is_dir():
        return 0
    cutoff = time.time() - days * 86400
    freed = 0
    for p in list(SCRATCH.iterdir()):
        seen = newest(p)
        if seen and seen < cutoff:
            freed += rm(p, dry_run)
    return freed


def key(path):
    """Windows compares paths case-insensitively; a set of strings must too."""
    return os.path.normcase(os.path.abspath(path))


#: What `tools/verify.py` builds, as the cargo commands its `build`, `test` and `clippy` steps
#: run, plus `build --workspace --all-targets` for every target kind whether verify compiles it
#: or not. A step whose arguments change there changes here, or what it builds stops counting as
#: live and is kept only by `GRACE_HOURS`.
LIVE_QUERIES = [
    ["build"],
    ["test", "--no-run"],
    ["clippy", "--all-targets", "--", "-D", "warnings"],
    ["build", "--workspace", "--all-targets"],
]


def live_artifacts():
    """Every file the graph `LIVE_QUERIES` builds actually uses, as cargo reports it.

    Asked rather than inferred -- see the module docstring on why age cannot answer this. A
    warm call is a no-op build that still emits a `compiler-artifact` line per unit, fresh or
    not. Returns None if any query fails, and the caller then leaves `deps/` untouched rather
    than deleting on a partial answer."""
    live = set()
    for query in LIVE_QUERIES:
        # `--message-format` is cargo's, so it goes ahead of the `--` that hands the rest to clippy.
        argv = ["cargo", query[0], "--message-format=json", *query[1:]]
        try:
            p = subprocess.run(argv, cwd=ROOT, capture_output=True, encoding="utf-8",
                               errors="replace")
        except OSError:
            return None
        if p.returncode != 0:
            return None
        for line in (p.stdout or "").splitlines():
            try:
                msg = json.loads(line)
            except ValueError:
                continue
            if msg.get("reason") != "compiler-artifact":
                continue
            for name in (msg.get("filenames") or []) + [msg.get("executable")]:
                if not name:
                    continue
                live.add(key(name))
                # On windows-msvc the debug info, the import library and the dep-info file sit
                # beside the artifact under the same stem, and cargo lists none of them.
                for ext in (".pdb", ".d", ".exp", ".lib"):
                    live.add(key(Path(name).with_suffix(ext)))
    return live


def dead_deps(live, grace_hours=GRACE_HOURS):
    """Files in target/debug/{deps,examples} that no live unit claims and nothing has written
    for `grace_hours`. [] if `live` is None.

    `debug` only: the live set is asked of the debug profile, so under any other profile every
    file would read as dead."""
    if live is None:
        return []
    cutoff = time.time() - grace_hours * 3600
    doomed = []
    for sub in ("deps", "examples"):
        d = TARGET / "debug" / sub
        if not d.is_dir():
            continue
        for p in d.iterdir():
            if not p.is_file() or key(p) in live:
                continue
            try:
                st = p.stat()
            except OSError:
                continue
            if st.st_mtime < cutoff:
                doomed.append((p, st.st_size))
    return doomed


def generations():
    """(files, distinct artifacts) across target/*/deps -- a free estimate of the pile-up.

    Filenames only, so the report can print it without running a build."""
    files = names = 0
    for d in TARGET.glob("*/deps"):
        stems = set()
        for p in d.iterdir():
            m = HASHED.match(p.stem)
            if not m:
                continue
            files += 1
            stems.add(m.group(1) + p.suffix)
        names += len(stems)
    return files, names


def mtime(path):
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def newest(path):
    """The latest mtime of `path` and of every file under it; 0.0 if it cannot be read.

    A directory's own mtime moves only when an entry directly inside it is added or removed, so
    a directory whose files are still being rewritten reads as old as the day it was made."""
    latest = mtime(path)
    if latest and path.is_dir():
        latest = max([latest, *(m for _, _, m in walk(path)[1])])
    return latest


def stale_incremental(keep=KEEP_INCREMENTAL, grace_hours=GRACE_HOURS):
    """Cache directories in target/*/incremental past the newest `keep` for their crate that no
    build has written for `grace_hours`.

    The same generation pile-up as `deps/`: `nvs_stdlib-034481himi0e6` is one unit's cache and
    `nvs_stdlib-0fcvo7f0ezdf2` another's. Decided by name and age rather than by asking cargo,
    because the hash here is an incremental session id that no artifact list ever mentions. Age
    carries it because a crate is built several ways at once -- lib, test, clippy, each `-p` and
    feature set, each with its own entry -- and a compile rewrites the entry it used, so the
    newest `keep` alone would condemn caches still in use. Deleting one costs its crate one
    non-incremental compile and nothing else, since nothing under this directory is an output."""
    cutoff = time.time() - grace_hours * 3600
    doomed = []
    for d in TARGET.glob("*/incremental"):
        groups = {}
        for entry in d.iterdir():
            groups.setdefault(entry.name.rsplit("-", 1)[0], []).append(entry)
        for entries in groups.values():
            entries.sort(key=mtime, reverse=True)
            doomed.extend(e for e in entries[keep:] if mtime(e) < cutoff)
    return doomed


def clean(keep_runs=KEEP_RUNS, scratch_days=SCRATCH_DAYS, keep_incremental=KEEP_INCREMENTAL,
          grace_hours=GRACE_HOURS, dry_run=False):
    """The whole of `--clean`, as {part: bytes freed}. `target/deps` is None when cargo could not
    name the live set, and `deps/` was then left alone.

    One entry point, so the command a person types and the sweep `tools/loop.py` runs at the end
    of every goal are the same code under the same policy."""
    live = live_artifacts()
    return {
        ".loop/logs": prune_logs(keep_runs, dry_run),
        ".agent-tmp": prune_scratch(scratch_days, dry_run),
        "target/deps": (None if live is None
                        else sum(rm(p, dry_run) for p, _ in dead_deps(live, grace_hours))),
        "target/incremental": sum(rm(p, dry_run)
                                  for p in stale_incremental(keep_incremental, grace_hours)),
    }


def total(freed):
    """The bytes a `clean` result freed, a part it left alone counting as none."""
    return sum(n for n in freed.values() if n)


def freed_lines(freed, verb="freed"):
    """One line per part of a `clean` result, printed alike by `--clean` and by the driver."""
    return [f"{name:<19} left alone -- cargo could not name the live set; does the tree build?"
            if n is None else f"{name:<19} {verb} {human(n)}" for name, n in freed.items()]


# -------------------------------------------------------------------------------- report


def report(deep=False):
    print(f"free on {ROOT.drive or '/'}  {free_gb():.1f}G"
          f"   (tools/loop.py refuses to start a run below {MIN_FREE_GB}G)")
    print()

    target_total = walk(TARGET)[0]
    files, names = generations()
    log_total = walk(LOGDIR)[0]
    scratch_total = walk(SCRATCH)[0]

    inc = walk(TARGET / "debug" / "incremental")[0] + walk(TARGET / "release" / "incremental")[0]
    stale_inc = sum(walk(p)[0] for p in stale_incremental())

    print("in this repository")
    print(f"  target/       {human(target_total):>8}   deps/: {files} files for {names} artifacts"
          f"{f' -- about {files / names:.1f} generations' if names else ''}")
    print(f"  {'':<12} {'':>8}   incremental/: {human(inc)}, of which {human(stale_inc)} "
          f"is past the newest {KEEP_INCREMENTAL} per crate and idle {GRACE_HOURS}h")
    print(f"  {'':<12} {'':>8}   `--clean` keeps what verify builds and anything written in the "
          f"last {GRACE_HOURS}h; release/deps is never swept")
    print(f"  .loop/logs    {human(log_total):>8}   "
          f"kept: newest {KEEP_RUNS} runs -- swept after every loop session")
    print(f"  .agent-tmp    {human(scratch_total):>8}   "
          f"kept: written within {SCRATCH_DAYS}d -- swept after every loop session")
    print()

    print("outside this repository -- reported, never touched by this script")
    home = Path.home().as_posix()
    tmp = "/var/tmp" if os.name != "nt" else "(WSL) /var/tmp"
    for name, path, what, how in ELSEWHERE:
        shown = path.format(home=home, tmp=tmp)
        size = f"{human(walk(Path(shown))[0]):>8}" if deep and Path(shown).is_dir() else "       ?"
        print(f"  {name:<20} {size}  {shown}")
        print(f"  {'':<20} {'':>8}  {what}")
        print(f"  {'':<20} {'':>8}  $ {how}")
    if not deep:
        print(f"  (sizes: --deep, which walks all {len(ELSEWHERE)} and is slow)")


# ---------------------------------------------------------------------------------- main


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--clean", action="store_true", help="sweep, rather than only report")
    ap.add_argument("-n", "--dry-run", action="store_true",
                    help="with --clean: say what would go, delete nothing")
    ap.add_argument("--deep", action="store_true", help="also size the paths outside this repo")
    ap.add_argument("--keep-runs", type=int, default=KEEP_RUNS)
    ap.add_argument("--scratch-days", type=int, default=SCRATCH_DAYS)
    ap.add_argument("--keep-incremental", type=int, default=KEEP_INCREMENTAL)
    ap.add_argument("--grace-hours", type=float, default=GRACE_HOURS,
                    help="keep anything in target/ written this recently, whatever cargo says; "
                         "0 sweeps everything cargo calls dead")
    opts = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    if not opts.clean:
        report(opts.deep)
        return 0

    # A driver is mid-run, so cargo may be writing target/ right now. Sweeping under a live
    # build is the one way this script could break something rather than merely cost a rebuild.
    # The driver runs this same `clean` itself after every session, where it knows none is.
    if RUNNING.exists():
        print(f"a loop driver holds {RUNNING.relative_to(ROOT).as_posix()} -- stop it first.")
        print("Sweeping target/ under a running build is the one thing here that is not safe;")
        print("the driver sweeps on its own after every session's acceptance check.")
        return 2

    before = free_gb()
    freed = clean(opts.keep_runs, opts.scratch_days, opts.keep_incremental, opts.grace_hours,
                  opts.dry_run)
    verb = "would free" if opts.dry_run else "freed"
    for line in freed_lines(freed, verb):
        print(f"  {line}")
    print(f"  {'total':<19} {verb} {human(total(freed))}"
          + ("" if opts.dry_run else f"; {before:.1f}G -> {free_gb():.1f}G free"))
    if not opts.dry_run and (freed["target/deps"] or freed["target/incremental"]):
        print("\nThe next build is slower by whatever it has to make again. That is the whole cost.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
