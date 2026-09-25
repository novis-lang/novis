#!/usr/bin/env python3
"""A side run: one goal, in a worktree of its own, that lands on `main` when it is green.

    python tools/loop.py --side <slug>            # prepare the worktree, run sessions, land, clean up
    python tools/loop.py --side <slug> --land     # no session: verify what is there, and land it
    python tools/side.py --status                 # every side goal, its worktree, and the merge lock

A side goal is `docs/agent/goals/side/<slug>.md` plus its `.toml` and `.handoff.md`
(`docs/agent/goals/README.md` § *Side goals* is the contract, `tools/goals.py` the reader). This
module is the half that is not reading: where a side run's worktree is, the handshake that lets it
move `main` under a running chain loop, and the git steps of landing. `tools/loop.py` is the caller
of all of it and keeps the parts that need a `Goal`.

**The run.** `loop.py --side <slug>` typed in the main tree is the *launcher*. It checks the goal's
files are committed, makes branch `side/<slug>` and its worktree at `.agent-tmp/worktrees/side/<slug>`
if either is missing (`prepare`), sets `goals.SIDE_ENV`, and hands the worktree's own
`tools/loop.py` to `tools/respawn.py` with the worktree as the working directory. Every tool of
every turn then roots itself at the worktree -- its `.loop/`, its `target/`, its `.agent-tmp/` -- and
reads the side goal's three files where a chain run reads the installed `docs/agent/loop-goal.*` and
`docs/agent/handoff.md`. So a side run and the chain run never share a state file, a build or a  # check-links:retired
working tree, and both can run at once. When the child exits having landed, the launcher removes
the worktree, the branch and the WSL target (`cleanup`), because only a process whose working
directory is outside the worktree can delete it on Windows.

**What a side run is checked against.** Its own checks, plus every check main's installed goal
already carries as its floor -- the worktree's `docs/agent/loop-goal.toml`, which each rebase brings  # check-links:retired
up to date (`effective_spec`). The live chain goal's own checks are left out: they are that goal's
unfinished work, and a side run held red on them could never land.

**Landing** is `loop.land`, in this order: rebase onto `main`; run `nv verify` and the full
acceptance list with the floor gate open; take `main/.loop/merge.lock`; wait until the chain loop
has yielded (or is not running, or is holding) and main's tracked files are clean; confirm `main`
has not moved since the rebase; carry the side goal's own checks into main's installed goal as floor
(`carry_into_live`), retire the side goal's `.toml` and `.handoff.md`, commit; fast-forward `main`;
release the lock. `main` moving in between costs one more rebase and verify, never a merge of
anything unverified, because the lock is only held for the steps after the verify.

**The handshake.** The side run writes `merge.lock` into main's `.loop/`. The chain loop checks for
it at the one boundary where it touches nothing -- before a session starts -- and while it stands
there it writes `yielded` and waits (`loop.merge_yield`). The side run moves `main` only while
`yielded` stands, or no chain loop holds `.loop/running`, or the chain loop is holding on a
`.loop/pause`. Each file names the process that wrote it, and one whose process is gone is stale and
ignored, so a crashed side run can never park the chain loop for good.

What it spends: a second checkout and a second `target/` for as long as the side run lives, and a
second WSL target dir; all three are deleted when it lands.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import goals as goalsmod  # noqa: E402  -- the side goal's files have one reader and it is there

ROOT = goalsmod.ROOT
IS_WINDOWS = os.name == "nt"

#: Main's installed goal, relative to a tree. The floor a side run carries, and the file its own
#: checks are carried into when it lands.
LIVE_GOAL = Path("docs") / "agent" / "loop-goal.toml"
#: Where the chain loop's state lives, relative to a tree.
RUNDIR = Path(".loop")
#: Written into MAIN's `.loop/` by the side run that wants to move `main`, and removed by it.
MERGE_LOCK = RUNDIR / "merge.lock"
#: Written into MAIN's `.loop/` by the chain loop while it waits on a `merge.lock`.
YIELDED = RUNDIR / "yielded"
#: Written into the WORKTREE's `.loop/` by a side run that landed, and read by its launcher.
LANDED = RUNDIR / "landed"
#: The reason the last landing attempt did not land, for the next session's prompt.
LANDING = RUNDIR / "side-landing.txt"
#: The chain loop's run marker and hold file, relative to a tree (`loop.RUNNING`, `loop.PAUSE`).
RUNNING = RUNDIR / "running"
PAUSE = RUNDIR / "pause"
#: The memo files a fresh worktree is seeded with. Both are keyed by content, so a verdict filed in
#: the main tree answers for the same bytes in the worktree and the first sweep is not a cold one.
MEMOS = (RUNDIR / "goal-green.json", RUNDIR / "check-reads.json")
#: Free space a new worktree needs on top of `--min-free-gb`: one debug and one release build.
WORKTREE_GB = 30
#: How often a landing waiting on `main` looks again.
POLL_SECONDS = 15


def git(cwd, *args, check=True):
    """`git <args>` in `cwd`, as its stripped stdout. Raises `RuntimeError` with git's own
    message when `check` and the command fails."""
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    if check and r.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {(r.stderr or r.stdout).strip()}")
    return r.stdout.strip()


def main_root(tree=ROOT):
    """The main worktree of the repository `tree` belongs to: the parent of the shared `.git`."""
    common = Path(git(tree, "rev-parse", "--path-format=absolute", "--git-common-dir"))
    return common.parent


def branch(slug):
    return f"side/{slug}"


def worktree(slug, main=None):
    return (main or main_root()) / ".agent-tmp" / "worktrees" / "side" / slug


def wsl_target(base, slug):
    """The side run's own WSL target dir: sharing the chain run's would lock and rebuild both."""
    return f"{base.rstrip('/')}-side-{slug}"


def pid_alive(pid):
    """Whether process `pid` is still running."""
    if pid <= 0:
        return False
    if IS_WINDOWS:
        import ctypes
        from ctypes import wintypes
        k32 = ctypes.WinDLL("kernel32", use_last_error=True)
        k32.OpenProcess.restype = wintypes.HANDLE
        k32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        handle = k32.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
        if not handle:
            return False
        try:
            code = wintypes.DWORD()
            ok = k32.GetExitCodeProcess(handle, ctypes.byref(code))
            return bool(ok) and code.value == 259  # STILL_ACTIVE
        finally:
            k32.CloseHandle(handle)
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def owner(path):
    """The live pid a marker file names on its `pid:` line, or 0 when absent, unreadable or dead."""
    try:
        for line in Path(path).read_text(encoding="utf-8").splitlines():
            if line.startswith("pid:"):
                pid = int(line.split(":", 1)[1].strip())
                return pid if pid_alive(pid) else 0
    except (OSError, ValueError):
        pass
    return 0


def write_marker(path, **fields):
    """Write a marker naming this process. Returns False when another live process holds it."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    held = owner(path)
    if held and held != os.getpid():
        return False
    body = f"pid:     {os.getpid()}\n" + "".join(f"{k + ':':<9}{v}\n" for k, v in fields.items())
    body += f"written: {datetime.now():%Y-%m-%d %H:%M:%S}\n"
    scratch = path.with_suffix(path.suffix + ".tmp")
    scratch.write_text(body, encoding="utf-8", newline="\n")
    os.replace(scratch, path)
    return True


def merge_requested(main):
    """The side run asking to move `main`, as `(pid, slug)`, or None when nobody live is asking."""
    path = main / MERGE_LOCK
    pid = owner(path)
    if not pid:
        return None
    slug = ""
    try:
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith("slug:"):
                slug = line.split(":", 1)[1].strip()
    except OSError:
        pass
    return pid, slug


def main_quiet(main):
    """Why the chain loop is not touching `main` right now, or "" when it may be.

    Three answers: no live chain loop holds `.loop/running`; it wrote `yielded` in answer to a
    lock; or it is holding on `.loop/pause` with its `held:` line written."""
    if not owner(main / RUNNING):
        return "no chain loop is running"
    if owner(main / YIELDED):
        return "the chain loop has yielded"
    try:
        if any(line.startswith("held:") for line in
               (main / PAUSE).read_text(encoding="utf-8").splitlines()):
            return "the chain loop is holding"
    except OSError:
        pass
    return ""


def main_dirty(main):
    """The tracked paths main's worktree has changed, or [] when a fast-forward can land."""
    # Unstripped: a porcelain line is `XY path`, and X is a space for an unstaged change.
    out = subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=main,
                         capture_output=True, text=True, encoding="utf-8", errors="replace").stdout
    return [line[3:] for line in out.splitlines() if line.strip()]


def effective_spec(side_toml, live_toml):
    """The acceptance list a side run is checked against, as TOML text: the side goal's own file
    with every check `live_toml` already carries as floor inserted at its marker.

    Read fresh for every check, so a session's edit to its own `.toml` and a rebase that moved
    main's floor are both what the next sweep runs."""
    carry = _goal_switch().carry
    text = Path(side_toml).read_text(encoding="utf-8")
    if not Path(live_toml).is_file():
        return text
    out, _, _ = carry(text, Path(live_toml).read_text(encoding="utf-8"),
                      LIVE_GOAL.as_posix(), carried_only=True)
    return out


def carry_into_live(side_toml, live_toml):
    """Insert the side goal's own checks into main's installed goal as floor. Returns how many.

    What makes a landed side goal protected the way a walked chain goal is: the chain carries
    everything in the installed goal into the next one at every switch."""
    gs = _goal_switch()
    live = Path(live_toml)
    out, floor, _ = gs.carry(live.read_text(encoding="utf-8"),
                             Path(side_toml).read_text(encoding="utf-8"),
                             goalsmod.rel(side_toml))
    if floor:
        live.write_text(out, encoding="utf-8", newline="\n")
    return len(floor)


def _goal_switch():
    """`tools/goal-switch.py` as a module: its name has a hyphen, so it cannot be imported."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("goal_switch",
                                                  Path(__file__).resolve().parent / "goal-switch.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def prepare(slug, main, free_gb, min_free_gb):
    """The worktree a side run works in, made if missing. Returns `(path, "")` or `(None, why)`.

    Refuses a goal whose files are not committed on `main`, since the branch would not have them,
    and a new worktree on a disk without room for its builds. An existing worktree is reused as it
    stands: that is how a stopped side run resumes."""
    goal = goalsmod.SideGoal(slug)
    for path in (goal.md, goal.toml, goal.handoff):
        rel = path.relative_to(main).as_posix()
        if not (main / rel).is_file():
            return None, f"{rel} does not exist -- a side goal is its .md, .toml and .handoff.md"
        if subprocess.run(["git", "cat-file", "-e", f"main:{rel}"], cwd=main,
                          capture_output=True).returncode != 0:
            return None, f"{rel} is not committed on main -- the side branch would not have it"
    wt = worktree(slug, main)
    if (wt / ".git").exists():
        return wt, ""
    if free_gb < min_free_gb + WORKTREE_GB:
        return None, (f"{free_gb:.0f} GB free, and a new worktree needs --min-free-gb "
                      f"({min_free_gb}) plus {WORKTREE_GB} for its own builds")
    wt.parent.mkdir(parents=True, exist_ok=True)
    exists = git(main, "branch", "--list", branch(slug))
    args = ["worktree", "add", str(wt), branch(slug)] if exists else \
        ["worktree", "add", "-b", branch(slug), str(wt), "main"]
    try:
        git(main, *args)
    except RuntimeError as e:
        return None, str(e)
    for memo in MEMOS:
        if (main / memo).is_file():
            (wt / memo).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(main / memo, wt / memo)
    return wt, ""


def cleanup(slug, main, wsl_dir=""):
    """Remove a landed side run's worktree, branch and WSL target. Returns what it could not."""
    left = []
    wt = worktree(slug, main)
    why = ""
    for _ in range(20):
        if not wt.exists():
            break
        # Never a forced form (AGENTS.md rule 10): a refusal means work would be lost, and that is
        # the person's question. Git can delete every file and then fail on the directory itself,
        # because a process that ran inside it -- a check's server, `cargo` -- is still exiting;
        # git has forgotten the worktree by then, and only an EMPTY directory is removed after it.
        if (wt / ".git").exists():
            r = subprocess.run(["git", "worktree", "remove", str(wt)], cwd=main,
                               capture_output=True, text=True)
            why = r.stderr.strip()
            if r.returncode != 0 and ("is dirty" in why or "untracked" in why):
                break  # uncommitted work: waiting will not change it
        else:
            try:
                wt.rmdir()
            except OSError:
                why = "files are still inside it after git removed the worktree"
        if wt.exists():
            time.sleep(3)
    if wt.exists():
        left.append(f"the worktree {wt} ({why})")
    else:
        try:
            wt.parent.rmdir()  # `worktrees/side/`, once its last side run has landed
        except OSError:
            pass
    git(main, "worktree", "prune", check=False)
    if git(main, "branch", "--list", branch(slug)):
        r = subprocess.run(["git", "branch", "-d", branch(slug)], cwd=main,
                           capture_output=True, text=True)
        if r.returncode != 0:
            left.append(f"the branch {branch(slug)} ({r.stderr.strip()})")
    if wsl_dir and shutil.which("wsl.exe" if IS_WINDOWS else "wsl"):
        subprocess.run(["wsl.exe" if IS_WINDOWS else "wsl", "--", "rm", "-rf", wsl_dir],
                       capture_output=True)
    return left


def status(main):
    """`--status`: every side goal, where its run stands, and who holds the merge lock."""
    goals = goalsmod.load_side()
    if not goals:
        print("no side goal under docs/agent/goals/side/")
    for g in goals:
        wt = worktree(g.slug, main)
        where = ("retired" if g.retired else
                 f"worktree {wt.relative_to(main).as_posix()}" if (wt / ".git").exists() else
                 "not started")
        print(f"  {g.slug:<28} {where}")
    asked = merge_requested(main)
    if asked:
        print(f"merge lock: held by side goal `{asked[1]}` (pid {asked[0]})")
    for err in goalsmod.side_errors():
        print(f"error: {err}")
    return 1 if goalsmod.side_errors() else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--status", action="store_true", help="every side goal and the merge lock")
    opts = ap.parse_args()
    if opts.status:
        return status(main_root())
    ap.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
