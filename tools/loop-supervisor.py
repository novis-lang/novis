#!/usr/bin/env python3
"""One layer above `tools/loop.py`: keeps a long unattended run going across driver restarts, and
spends one session every few dozen on the loop itself.

**Every flag is loop.py's and is passed through untouched**, so this is invisible: swap the script
name and nothing about a run changes except that it now survives its own improvements.

    python tools/loop-supervisor.py --max-sessions 300 --effort medium
    python tools/loop-supervisor.py --max-sessions 300 --chain docs/agent/goals/chain.toml --effort medium
    python tools/loop-supervisor.py --optimize-only     # run the pass now, against the tree as it is
    python tools/loop-supervisor.py --no-optimize       # just the restarts

Two things it exists for, and only one of them is the optimization pass:

* **`loop.py` is the one piece of the loop that does not hot-reload.** `orient.py` is a subprocess,
  `session-prompt.md` is re-read and `loop-goal.toml` is re-loaded every session, so a session that
  improves any of those improves the *next* session. The driver holds its own code, so a session
  that improves the driver improves nothing until somebody restarts it by hand. This restarts it.
* **A loop changes the shape of its own input.** The pack grew 59 KB -> 118 KB at +907 B a session
  once, re-billed on all ~81 calls of every session after it, and the projected slice cap fell to
  one on the strength of that alone. Nothing announced it. `docs/agent/optimization-prompt.md` is
  the pass that reverses it; this decides when to run one.

**A leg is a `loop.py` run.** The supervisor cuts `--max-sessions` into legs, restarts the driver
between them, and at a leg boundary decides whether the loop has drifted enough to spend a session
on itself. It restarts on exactly one verdict -- `kind: "budget"` in `.loop/run-end.json`, meaning
the driver served its sessions and stopped. Every other verdict is terminal, including the ones
that look recoverable: a stall streak, a CLI that keeps failing and a usage window that never
reopened are all reasons a *human* should look, and a supervisor that retried them would turn one
bad hour into eight.

Nothing here judges the work. The stop path is still exit codes and exact output matching inside
`loop.py`, and this adds no verdict of its own to it.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import threading
import time
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import disk  # noqa: E402  -- same directory; the retention policy has one home and it is there
import loop  # noqa: E402  -- the driver. Its colours, its ledger and its renderer, not a second copy

ROOT = loop.ROOT
RUNDIR = loop.RUNDIR
LOGDIR = loop.LOGDIR
STOP = loop.STOP
RUNEND = loop.RUNEND

MARKER = RUNDIR / "supervisor"
OPTDIR = RUNDIR / "optimization"
OPTSTATE = OPTDIR / "state.json"
OPTSTATUS = RUNDIR / "optimize-status.txt"
PACKLOG = RUNDIR / "pack-size.jsonl"
OPT_PROMPT = ROOT / "docs" / "agent" / "optimization-prompt.md"

C = loop.C
say = loop.say
mmss = loop.mmss

#: The only verdict a leg may end on and be restarted. See `loop.write_run_end` for the full set.
RESTARTABLE = "budget"

#: One-shot `loop.py` modes. They print something and exit; cutting them into legs is nonsense, so
#: the supervisor gets out of the way entirely and becomes a spelling of `loop.py`. `--help` is
#: deliberately not here: this script's own flags would then be undiscoverable, and its epilog
#: names `loop.py --help` for the rest.
ONESHOT = ("--goal-only", "--leg-only", "--list", "--chain-install")

#: What an optimization pass may commit. Everything outside this is reverted, unread: the pass is
#: the loop working on itself, and `crates/`, `tests/` and `examples/` are the work, not the loop.
#: An allowlist rather than a denylist because a new top-level directory must default to refused.
ALLOWED = ("tools/", "docs/", "AGENTS.md", "CLAUDE.md", "README.md")

#: A pass only runs when a signal fires, so the cadence below is when the supervisor *looks*, not
#: how often it spends a session. Looking is four subprocesses; the pass is a session.
DEFAULT_PROBE_EVERY = 10
DEFAULT_OPTIMIZE_EVERY = 25
DEFAULT_MIN_GAP = 15
DEFAULT_PACK_GROWTH = 20 * 1024

#: How the pack-growth signal opens. It is the one signal that can bring a pass *forward*, so the
#: check for it reads this rather than sniffing for a word: a reworded signal would otherwise
#: disable the early trigger silently, which is the failure this whole file exists to catch.
PACK_SIGNAL = "the orientation pack grew"


# ----------------------------------------------------------------------------- small helpers


def stamp() -> str:
    return f"{datetime.now():%Y%m%d-%H%M%S}"


def rel(path: Path) -> str:
    return loop.rel_to_root(path)


def clip(text: str, head: int = 40, tail: int = 140) -> str:
    """Bound a probe's output without losing either end.

    Head *and* tail, because these tools put the table first and the conclusion last:
    `loop-stats.py` opens with a row per session and closes with the constants and the projection,
    which is the half the pass actually reads. A plain head-truncation would drop exactly that."""
    lines = text.rstrip().split("\n")
    if len(lines) <= head + tail:
        return "\n".join(lines)
    hidden = len(lines) - head - tail
    return "\n".join(lines[:head] + [f"   ... [{hidden} lines omitted] ...", ""] + lines[-tail:])


def probe(*argv, timeout=300) -> tuple[int, str]:
    """Run one of the loop's own measuring tools and return `(exit code, output)`.

    Never raises. A probe that cannot run is a missing section in the evidence pack and a line in
    the report; it is never the reason a run of 300 sessions stops."""
    try:
        done = subprocess.run(
            [sys.executable, str(ROOT / "tools" / argv[0]), *argv[1:]],
            cwd=ROOT, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=timeout,
        )
    except (OSError, subprocess.SubprocessError) as e:
        return 127, f"(did not run: {e})"
    return done.returncode, (done.stdout or "") + (done.stderr or "")


def git(*args) -> str:
    return loop.git(*args)


def git_ok(*args) -> bool:
    """Did this git command succeed? `loop.git` returns stdout and swallows the status, which is
    right for reading a rev and useless for the one place here that must know whether a `revert`
    actually applied."""
    try:
        return subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, encoding="utf-8"
        ).returncode == 0
    except OSError:
        return False


def verify_state() -> tuple[bool, str]:
    """`(green, the step it failed at)`.

    A gate that demanded green outright would roll back every pass whenever the *language* was red
    -- and the language is red most of the time, because a red acceptance check is what the loop is
    working on. So the pass is judged against the tree as it was handed over, not against an ideal.
    `verify.py` prints `verify: FAILED at <step>`, which is the whole of what a comparison needs."""
    code, out = probe("verify.py", timeout=3600)
    if code == 0:
        return True, ""
    for line in out.split("\n"):
        if line.startswith("verify: FAILED at "):
            return False, line[len("verify: FAILED at "):].split()[0]
    return False, f"exit {code}"


def tools_still_load(changed) -> str:
    """Every changed `tools/*.py` parses and answers `--help`. Empty string when they all do.

    The cheap half of the code gate, and the half that catches what actually goes wrong: a syntax
    error or an argparse mistake in a script the driver shells out to. It costs a second and it
    does not care what state the Rust tree is in, so unlike `verify.py` it is conclusive."""
    for path in [p for p in changed if p.startswith("tools/") and p.endswith(".py")]:
        if not (ROOT / path).exists():  # the pass deleted it; that is the allowlist's business
            continue
        try:
            done = subprocess.run(
                [sys.executable, "-m", "py_compile", str(ROOT / path)],
                cwd=ROOT, capture_output=True, text=True, timeout=60,
            )
        except (OSError, subprocess.SubprocessError) as e:
            return f"{path} could not be compiled: {e}"
        if done.returncode != 0:
            return f"{path} does not parse: {(done.stderr or '').strip()[:200]}"
        code, out = probe(Path(path).name, "--help", timeout=120)
        if code != 0:
            return f"`python {path} --help` exits {code}: {out.strip()[:200]}"
    return ""


def read_json(path: Path, default=None):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return default


def write_json(path: Path, obj) -> None:
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(obj, indent=2) + "\n", encoding="utf-8", newline="\n")
    except OSError:
        pass


def pack_bytes() -> int:
    """The size of the orientation pack as it stands, out of the log `session.py --wrap` keeps.

    Read from the log rather than measured here because measuring means running `orient.py`, and
    the number wanted is the one the *sessions* were charged, not the one a probe would be."""
    last = 0
    for line in (PACKLOG.read_text(encoding="utf-8").splitlines()
                 if PACKLOG.exists() else []):
        if line.strip():
            try:
                last = int(json.loads(line).get("bytes") or last)
            except ValueError:
                pass
    return last


# --------------------------------------------------------------------------------- the leg


def child_argv(opts, passthrough, sessions) -> list[str]:
    """`loop.py`'s argv for one leg. The flags this script understands are rebuilt verbatim; every
    other one is handed on in the order it was typed, so a flag added to `loop.py` tomorrow works
    here today without this file knowing it exists."""
    argv = [sys.executable, str(ROOT / "tools" / "loop.py"),
            "--max-sessions", str(sessions),
            "--model", opts.model,
            "--permission-mode", opts.permission_mode,
            "--keep-runs", str(opts.keep_runs)]
    if opts.effort:
        argv += ["--effort", opts.effort]
    if opts.force:
        argv.append("--force")
    return argv + list(passthrough)


def run_leg(argv) -> tuple[int, dict]:
    """One `loop.py` run, with this console handed straight to it.

    Stdio is **inherited, never captured**: `s` and `r` are read from the console by the driver,
    the status line is painted on the bottom row, and both stop working the moment anything sits
    in between. That is the whole of what "the supervisor is invisible" means in practice.

    A Ctrl-C reaches the driver and this process at the same instant. The driver's handler writes
    its ledger line and `run-end.json` -- with the sessions it served -- and needs more than the
    quarter second `subprocess.run` would give it before killing the child, so this waits for it
    and only then lets the interrupt go on up to `supervise`, which reads that file."""
    proc = subprocess.Popen(argv, cwd=ROOT)
    try:
        proc.wait()
    except KeyboardInterrupt:
        try:
            proc.wait(timeout=20)
        except subprocess.TimeoutExpired:
            proc.kill()
        raise
    end = read_json(RUNEND, default={}) or {}
    if not end:
        end = {"kind": "unknown", "reason": f"{rel(RUNEND)} was not written", "served": 0}
    return proc.returncode, end


# ------------------------------------------------------------------------------- the signals


def gather_signals(state) -> tuple[list[str], dict]:
    """What has drifted since the last pass, and the evidence for saying so.

    Returns the signals that fired and the probe output behind them. **No signal, no pass** --
    that is what makes the cadence cheap enough to be wrong about: looking costs four
    subprocesses, and a pass that would have found nothing is a session not spent."""
    fired: list[str] = []
    ev: dict[str, str] = {}

    grown = pack_bytes() - int(state.get("pack_bytes") or 0)
    ev["pack"] = (f"pack now {pack_bytes():,} B, {grown:+,} B since the last pass "
                  f"({state.get('pack_bytes') or 'never taken'})")
    if state.get("pack_bytes") and grown >= DEFAULT_PACK_GROWTH:
        fired.append(f"{PACK_SIGNAL} {grown:,} B since the last pass")

    code, out = probe("orient.py", "--audit")
    warnings = [ln for ln in out.split("\n") if ln.startswith("!! orient.py:")]
    tail = out.split("== WHAT THIS PACK COST", 1)
    ev["audit"] = ("== WHAT THIS PACK COST" + tail[1]) if len(tail) > 1 else "(no audit section)"
    ev["warnings"] = "\n".join(warnings) or "  none -- every selector resolves"
    if code != 0:
        fired.append(f"orient.py exits {code}: the pack the loop runs on does not build")
        ev["warnings"] += f"\n  orient.py exited {code}"
    if warnings:
        fired.append(f"{len(warnings)} dead selector(s) or stale anchor(s) in the pack")

    _, out = probe("playbook.py", "--dupes")
    ev["dupes"] = clip(out, 8, 60)
    if "none at this threshold" not in out:
        fired.append("the playbook says the same thing twice")

    _, out = probe("playbook.py", "--check")
    ev["playbook_check"] = clip(out, 8, 60)
    if "none -- every path any bullet names still exists" not in out:
        fired.append("a playbook bullet names a path that is no longer in the tree")

    code, out = probe("check-links.py")
    ev["links"] = clip(out, 8, 60)
    if code != 0:
        fired.append("check-links.py reports a dead link")

    # Not a signal of its own -- it fires nothing and cannot. It is what tells the pass *where* the
    # fix for the signals above goes, once the loop is walking goals a tool wrote.
    ev["generated"] = generated_by()

    return fired, ev


#: A goal file's own banner, when a tool wrote it rather than a person. What the pass needs is not
#: the fact that it is generated but the *command that regenerates it*, because that command is
#: where the fix goes -- see the optimization prompt's menu item 8.
GENERATED_RE = re.compile(r"^#\s*GENERATED by `([^`]+)`", re.M)


def generated_by() -> str:
    """The command that wrote the live goal, or "" when a person did."""
    try:
        found = GENERATED_RE.search(loop.GOAL_TOML.read_text(encoding="utf-8"))
    except OSError:
        return ""
    return found.group(1) if found else ""


def evidence_pack(fired, ev, since, report_path, baseline) -> str:
    """Everything the pass would otherwise spend ten calls fetching, piped in on its stdin.

    The same trick, and for the same measured reason, as the driver piping `orient.py` to a work
    session: a result this size comes back through a tool call as a spill notice and a readback,
    which costs more than the text. It also makes the pass *deterministic* -- it chooses among
    findings it was handed rather than deciding what to go and look at."""
    _, stats = probe("loop-stats.py", timeout=600)
    _, attrib = probe("loop-stats.py", "--attribute", timeout=600)
    free = disk.free_gb(ROOT)

    parts = [
        "You are the loop optimization pass. Your evidence follows; the prompt after it says what",
        "you may do with it. Do not re-run any of these to start with -- that is why they are here.",
        "",
        f"== WRITE YOUR REPORT TO: {rel(report_path)}",
        "",
        "== SIGNALS THAT FIRED",
        *(f"  - {s}" for s in fired),
        "",
        f"== THE {since} SESSION(S) SINCE THE LAST PASS",
        clip(ledger_since(since), 0, 80),
        "",
        "== WHAT THE SESSIONS COST  (python tools/loop-stats.py)",
        clip(stats),
        "",
        "== WHERE THE CONTEXT WENT  (python tools/loop-stats.py --attribute)",
        clip(attrib),
        "",
        "== WHAT THE PACK COST  (python tools/orient.py --audit)",
        ev.get("audit", ""),
        "",
        "== THE PACK'S SLOPE",
        f"  {ev.get('pack', '')}",
        "",
        "== SELECTOR WARNINGS  (python tools/orient.py)",
        ev.get("warnings", ""),
        "",
        "== THE LIVE GOAL",
        (f"  GENERATED by `{ev['generated']}`. Menu item 8 applies: a finding in it is a defect in\n"
         f"  the emitter, fixed there and re-emitted. A hand-edit is discarded by the next emission."
         if ev.get("generated") else
         "  hand-written -- a finding in it is fixed in it, the ordinary case"),
        "",
        "== DUPLICATE PLAYBOOK BULLETS  (python tools/playbook.py --dupes)",
        ev.get("dupes", ""),
        "",
        "== STALE PLAYBOOK PATHS  (python tools/playbook.py --check)",
        ev.get("playbook_check", ""),
        "",
        "== BROKEN LINKS  (python tools/check-links.py)",
        ev.get("links", ""),
        "",
        "== DISK",
        f"  {free:.1f}G free; the driver refuses a run below {disk.MIN_FREE_GB}G",
        "",
        "== THE TREE'S VERIFICATION STATE GOING IN",
        ("  green -- tools/verify.py passes, so any red after your pass is yours"
         if baseline[0] else
         f"  RED, at the `{baseline[1]}` step, before you touched anything. That is the loop's own\n"
         "  worklist and it is NOT yours to fix -- do not take a checklist item. It does mean your\n"
         "  pass is judged on whether it made this worse, not on whether it is green."),
        "",
    ]
    return "\n".join(parts)


def ledger_since(n: int) -> str:
    """The ledger's last few lines: what the sessions since the last pass actually reported.

    Bounded by the pass count rather than by the file, because `.loop/log.md` is the whole run
    history and only the recent end of it says anything about the drift being looked at."""
    try:
        lines = loop.LEDGER.read_text(encoding="utf-8").rstrip().split("\n")
    except OSError:
        return "  (no ledger)"
    return "\n".join(lines[-(n * 3 + 12):])


# ---------------------------------------------------------------------------------- the pass


def run_pass(opts, fired, ev, since, baseline) -> str:
    """One `claude` session against `docs/agent/optimization-prompt.md`, then the smoke test.

    Returns a one-line verdict for the ledger. The pass is the one place in this whole loop where
    an agent edits the machinery that will drive the next several hours unattended, so what
    follows it is not a review -- it is a set of exit codes, and a `git revert` on any of them."""
    OPTDIR.mkdir(parents=True, exist_ok=True)
    run = stamp()
    report = OPTDIR / f"{run}-report.md"
    log = LOGDIR / f"{run}-optimize.log"
    OPTSTATUS.unlink(missing_ok=True)

    base = git("rev-parse", "HEAD")
    dirty = git("status", "--porcelain").strip()
    if dirty:
        say("the tree is not clean going into the pass -- skipping it rather than mixing "
            "somebody's edits into a revert range", C.YELLOW)
        return "SKIPPED (tree not clean)"

    pack = evidence_pack(fired, ev, since, report, baseline)
    (OPTDIR / f"{run}-evidence.md").write_text(pack, encoding="utf-8", newline="\n")
    prompt = OPT_PROMPT.read_text(encoding="utf-8")

    exe = shutil.which("claude") or "claude"
    cmd = [exe, "-p", prompt, "--model", opts.model,
           "--permission-mode", opts.permission_mode,
           "--output-format", "stream-json", "--verbose"]
    if opts.effort:
        cmd += ["--effort", opts.effort]

    say(f"optimization pass {run}: {len(fired)} signal(s), evidence {len(pack):,} B", C.CYAN)
    for s in fired:
        say(f"   - {s}", C.GRAY)
    loop.CONSOLE.open_session(log)
    renderer = loop.Renderer(opts)
    started = time.monotonic()
    proc = subprocess.Popen(
        cmd, cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=None,
        encoding="utf-8", errors="replace", bufsize=1,
    )
    threading.Thread(target=loop.feed, args=(proc.stdin, pack), daemon=True).start()
    try:
        assert proc.stdout is not None
        for line in proc.stdout:
            renderer.event(line)
        proc.wait()
    except BaseException:
        # The same rule as a work session: an agent with permissions bypassed does not outlive
        # the process watching it.
        proc.kill()
        try:
            proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            pass
        raise
    finally:
        loop.CONSOLE.close_session()
    say(f"pass ended after {mmss(time.monotonic() - started)}, claude exit {proc.returncode}",
        C.CYAN)

    status = ""
    if OPTSTATUS.exists():
        status = OPTSTATUS.read_text(encoding="utf-8").strip()
    if proc.returncode != 0:
        # A refused or crashed pass has usually committed nothing, and a usage wall is the likeliest
        # cause of both. Judge the tree, not the exit code, then hand the wall back to `loop.py`,
        # which is the only thing here that knows how to wait one out.
        say(f"the pass exited {proc.returncode} -- checking what it left behind", C.YELLOW)

    verdict = settle(base, run, report, status, baseline)
    write_json(OPTSTATE, {
        "last_pass": run,
        "last_pass_head": git("rev-parse", "HEAD"),
        "pack_bytes": pack_bytes(),
        "verdict": verdict,
        "signals": fired,
    })
    return verdict


def settle(base, run, report, status, baseline) -> str:
    """Decide whether what the pass committed may stay, and undo it if not.

    Five gates, in the order a failure is cheapest to find in: what it touched, whether it left
    anything behind, whether the loop still orients and reads its goal, whether every script it
    edited still loads, and -- only when it edited code the loop executes -- whether the tree got
    worse. Nothing here reads the diff. The point is that the decision is mechanical, because the
    thing being judged is an agent's edit to the judge."""
    head = git("rev-parse", "HEAD")
    if head == base:
        left = git("status", "--porcelain").strip()
        if left:
            git("stash", "push", "-u", "-m", f"optimization pass {run}: uncommitted")
            say("the pass left uncommitted edits and no commit -- stashed them", C.YELLOW)
            return "STASHED (edits without a commit)"
        return status or "CLEAN (nothing committed)"

    shas = [s for s in git("rev-list", f"{base}..{head}").split("\n") if s]
    changed = [p for p in git("diff", "--name-only", base, head).split("\n") if p]
    say(f"the pass committed {len(shas)} change(s) over {len(changed)} file(s)", C.GRAY)

    stray = [p for p in changed if not p.startswith(ALLOWED)]
    if stray:
        return rollback(base, shas, f"it touched {', '.join(stray[:4])}"
                        + (f" and {len(stray) - 4} more" if len(stray) > 4 else ""))

    leftover = git("status", "--porcelain").strip()
    if leftover:
        git("stash", "push", "-u", "-m", f"optimization pass {run}: uncommitted tail")
        say("stashed an uncommitted tail the pass left behind", C.YELLOW)

    code, out = probe("orient.py")
    if code != 0 or len(out) < 500:
        return rollback(base, shas, f"orient.py exits {code} with {len(out)} bytes of pack")
    code, _ = probe("loop.py", "--list")
    if code != 0:
        return rollback(base, shas, f"loop.py --list exits {code}: the acceptance list is unreadable")

    broken = tools_still_load(changed)
    if broken:
        return rollback(base, shas, broken)

    if any(p.startswith("tools/") for p in changed):
        say("the pass edited tools/ -- verifying before handing the loop back", C.CYAN)
        ok, step = verify_state()
        if not ok and baseline[0]:
            return rollback(base, shas, f"verify.py was green going in and now FAILS at {step}")
        if not ok and step != baseline[1]:
            return rollback(base, shas, f"verify.py failed at {baseline[1]} going in and now "
                                        f"fails at {step} instead")
        if not ok:
            say(f"verify.py still fails at {step}, exactly as it did before the pass -- not the "
                "pass's doing, so it stands", C.YELLOW)

    say(f"the pass stands: {status or '(no status line)'}", C.GREEN)
    if report.exists():
        say(f"report: {rel(report)}", C.GRAY)
    return status or f"APPLIED {len(shas)}"


def rollback(base, shas, why) -> str:
    """Put the tree back, loudly.

    `revert` rather than `reset`, so the run's history still shows the pass and what was undone --
    and a `reset` only as the fallback for a revert that conflicts, which it can only do against
    commits the pass itself made, since no other writer runs between two legs."""
    say("", C.RED)
    say(f"ROLLING BACK the optimization pass: {why}", C.RED)
    ok = bool(shas)
    for sha in shas:  # rev-list is newest first, which is the order a revert must take
        if not git_ok("revert", "--no-edit", "--no-commit", sha):
            ok = False
            break
    if ok:
        ok = git_ok("commit", "-m",
                    f"revert: the optimization pass was rolled back -- {why}")
    if not ok:
        git("revert", "--abort")
        git("reset", "--hard", base)
        say("the revert did not apply cleanly; reset to the pre-pass commit instead", C.YELLOW)
    say("the loop continues on the code it had before the pass", C.YELLOW)
    return f"REVERTED ({why.splitlines()[0]})"


# ------------------------------------------------------------------------------- the marker


def claim() -> bool:
    if MARKER.exists():
        say(f"a supervisor is already up on this tree, per {rel(MARKER)}:", C.RED)
        for line in MARKER.read_text(encoding="utf-8").rstrip("\n").split("\n"):
            say(f"  {line}", C.RED)
        say("\nIf that one is actually over, delete the file and start again, or pass --force.",
            C.YELLOW)
        return False
    RUNDIR.mkdir(parents=True, exist_ok=True)
    MARKER.write_text(
        f"pid:      {os.getpid()}\n"
        f"host:     {platform.node()}\n"
        f"started:  {datetime.now():%Y-%m-%d %H:%M:%S}\n",
        encoding="utf-8", newline="\n",
    )
    return True


# ---------------------------------------------------------------------------------- driving


def checkpoint(opts, since) -> int:
    """A leg boundary. Look for drift; spend a session on it only if something is actually wrong.

    Returns the session count to carry forward -- 0 when the counter was spent, `since` when it
    was not, because a look that found nothing must not reset the clock on the next look. This
    owns every write to `state.json`, so the count and the pack size it is measured against can
    never be written from two different reads of the same file."""
    if opts.no_optimize:
        return since
    state = read_json(OPTSTATE, default={}) or {}
    say("")
    say(f"== checkpoint: {since} session(s) since the last pass", C.MAGENTA)
    # Scratch is free to drop; `sweep_target` is not -- it costs a full rebuild of whatever cargo
    # still wanted, which the next session then pays for inside its acceptance check. So it runs
    # only where the alternative is worse: below the floor the driver refuses to start at.
    freed = disk.prune_scratch()
    free = disk.free_gb(ROOT)
    if free < disk.MIN_FREE_GB:
        say(f"{free:.1f}G free, below the {disk.MIN_FREE_GB}G a run needs -- sweeping target/, "
            "which costs a rebuild", C.YELLOW)
        freed += disk.sweep_target()
    elif free < 2 * disk.MIN_FREE_GB:
        say(f"{free:.1f}G free; `python tools/disk.py --clean` is what reclaims the rest", C.YELLOW)
    if freed:
        say(f"reclaimed {disk.human(freed)}", C.GRAY)

    fired, ev = gather_signals(state)
    due = since >= opts.optimize_every
    early = since >= opts.min_pass_gap and any(s.startswith(PACK_SIGNAL) for s in fired)
    if not fired:
        say("nothing has drifted -- no pass", C.GREEN)
        loop.ledger(f"## supervisor checkpoint {datetime.now():%Y-%m-%d %H:%M} -- "
                    f"clean after {since} session(s), no pass")
        # A clean look still spends the clock: the next one is a full cadence away, and the pack
        # it will compare against is this one, not the one the last *pass* left.
        write_json(OPTSTATE, {**state, "since": 0 if due else since,
                              "pack_bytes": pack_bytes() if due else state.get("pack_bytes")})
        return 0 if due else since
    if not (due or early):
        say(f"{len(fired)} signal(s), but only {since} of {opts.optimize_every} sessions in -- "
            "carrying them to the next checkpoint", C.GRAY)
        for s in fired:
            say(f"   - {s}", C.GRAY)
        # Ledgered like the other two outcomes. This branch used to be silent, and a run whose
        # every checkpoint took it left no trace that the cadence had fired at all.
        loop.ledger(f"## supervisor checkpoint {datetime.now():%Y-%m-%d %H:%M} -- "
                    f"{len(fired)} signal(s) after {since} session(s), carried: "
                    + "; ".join(fired))
        write_json(OPTSTATE, {**state, "since": since})
        return since

    dirty = git("status", "--porcelain").strip()
    if dirty:
        # Somebody is editing this tree by hand, which the loop allows. A pass over a dirty tree
        # would mix their edits into its revert range, so it waits -- and the counter waits with
        # it, so the next leg is one session long and this is asked again straight after. It does
        # not reset: a pass deferred is not a pass taken. Checked before the baseline measurement
        # below, which is a full `verify.py` and worth nothing if the pass is not going to run.
        say(f"{len(fired)} signal(s) and the pass is due, but the tree is not clean -- "
            "deferring it until the next leg boundary", C.YELLOW)
        for s in fired:
            say(f"   - {s}", C.GRAY)
        loop.ledger(f"## supervisor checkpoint {datetime.now():%Y-%m-%d %H:%M} -- "
                    f"{len(fired)} signal(s) after {since} session(s), pass DEFERRED: "
                    f"the tree is not clean ({len(dirty.splitlines())} path(s))")
        write_json(OPTSTATE, {**state, "since": since})
        return since

    say("measuring the tree before the pass, so it is judged on what it changed", C.GRAY)
    baseline = verify_state()
    say(f"   verify.py going in: {'green' if baseline[0] else 'RED at ' + baseline[1]}",
        C.GREEN if baseline[0] else C.YELLOW)
    verdict = run_pass(opts, fired, ev, since, baseline)
    loop.ledger(f"## optimization pass {datetime.now():%Y-%m-%d %H:%M} after {since} session(s) "
                f"-- {verdict}")
    if verdict.startswith("BROKEN"):
        raise SystemExit(f"the optimization pass reported {verdict}")
    if verdict.startswith("SKIPPED"):
        return since  # not taken, so not spent -- the same deferral as the dirty-tree branch
    return 0


def leg_size(opts, remaining, since) -> int:
    """How many sessions the next leg gets.

    The smallest of three: what is left of `--max-sessions`, one probe leg, and the distance to
    the next cadence checkpoint. The third is what makes a checkpoint land *on* `--optimize-every`
    rather than at the first multiple of the probe leg past it."""
    return max(1, min(remaining, opts.probe_every, max(1, opts.optimize_every - since)))


def credit(since, end) -> int:
    """Add a leg's served sessions to the count since the last pass, and persist it.

    The one write to `since` that is not a checkpoint's, so a checkpoint that finds nothing and
    a leg that ends -- either way -- agree on the number the next checkpoint reads."""
    since += int(end.get("served") or 0)
    state = read_json(OPTSTATE, default={}) or {}
    write_json(OPTSTATE, {**state, "since": since})
    return since


def supervise(opts, passthrough) -> int:
    served_total = 0
    since = int((read_json(OPTSTATE, default={}) or {}).get("since") or 0)
    leg = 0
    # Everything this process says between legs -- the checkpoints above all -- was printed and
    # kept nowhere: the driver's console log belongs to a leg, and a leg is over by then. One file
    # per supervised run, on the same tee the driver uses, so "did the cadence fire" has a record.
    LOGDIR.mkdir(parents=True, exist_ok=True)
    loop.CONSOLE.open_run(LOGDIR / f"{stamp()}-supervisor.log")

    while served_total < opts.max_sessions:
        if STOP.exists():
            say(f"{rel(STOP)} is present -- not starting another leg", C.YELLOW)
            break
        remaining = opts.max_sessions - served_total
        size = leg_size(opts, remaining, since)
        leg += 1
        say("")
        say(f"== leg {leg}: up to {size} session(s), {remaining} of {opts.max_sessions} left",
            C.MAGENTA)
        before = read_json(RUNEND, default={}) or {}
        try:
            code, end = run_leg(child_argv(opts, passthrough, size))
        except KeyboardInterrupt:
            # The driver clears `run-end.json` when it starts and its Ctrl-C handler rewrites it
            # with the sessions served, so a file that differs from the one seen going in is this
            # leg's. Count those, then let the interrupt finish the run as before.
            end = read_json(RUNEND, default={}) or {}
            if end and end != before:
                since = credit(since, end)
            raise
        served = int(end.get("served") or 0)
        served_total += served
        since = credit(since, end)

        if end.get("kind") != RESTARTABLE:
            say("")
            say(f"the run ended for good: {end.get('reason') or f'loop.py exit {code}'}", C.YELLOW)
            return 0 if code == 0 else code
        if STOP.exists():
            say(f"{rel(STOP)} is present -- stopping", C.YELLOW)
            break
        if served_total >= opts.max_sessions:
            # No checkpoint on the way out. A pass exists to make the *next* sessions cheaper,
            # and there are none.
            break
        since = checkpoint(opts, since)

    say("")
    say(f"supervisor done: {served_total} session(s) over {leg} leg(s)", C.CYAN)
    return 0


def main() -> int:
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass

    argv = sys.argv[1:]
    if any(a in ONESHOT for a in argv):
        # `--list`, `--goal-only`, `--leg-only`, `--chain-install`, `--help`: nothing to supervise.
        return subprocess.run(
            [sys.executable, str(ROOT / "tools" / "loop.py"), *argv], cwd=ROOT
        ).returncode

    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="Every flag not listed here is loop.py's and is passed straight through; "
               "`python tools/loop.py --help` is the rest of this help.",
    )
    # loop.py's, declared here only because a leg needs them rebuilt. Defaults match loop.py's.
    ap.add_argument("--max-sessions", type=int, default=1)
    ap.add_argument("--model", default="opus")
    ap.add_argument("--effort", default=None,
                    choices=("low", "medium", "high", "xhigh", "max"))
    ap.add_argument("--permission-mode", default="bypassPermissions")
    ap.add_argument("--force", action="store_true")
    ap.add_argument("--keep-runs", type=int, default=max(disk.KEEP_RUNS, 20),
                    help="how many runs' logs survive each leg's prune. Raised above loop.py's "
                         "default because a leg IS a run: at the default a supervised run would "
                         "leave loop-stats.py five legs of transcripts instead of five runs'")
    # The renderer's, so a pass looks like a session on the console.
    ap.add_argument("--max-result-lines", type=int, default=60)
    ap.add_argument("--max-input-lines", type=int, default=40)
    ap.add_argument("--max-line-chars", type=int, default=500)
    ap.add_argument("--full-output", action="store_true")
    # This script's own.
    ap.add_argument("--optimize-every", type=int, default=DEFAULT_OPTIMIZE_EVERY, metavar="N",
                    help="sessions between checkpoints that may spend a session on the loop "
                         "itself. A checkpoint with no signal costs four subprocesses, not a "
                         "session, which is what makes this number safe to be wrong about")
    ap.add_argument("--probe-every", type=int, default=DEFAULT_PROBE_EVERY, metavar="N",
                    help="sessions per leg. Every boundary restarts loop.py -- which is how a "
                         "driver change committed by a work session takes effect -- and is where "
                         "an early pass can be triggered by drift rather than by the count")
    ap.add_argument("--min-pass-gap", type=int, default=DEFAULT_MIN_GAP, metavar="N",
                    help="never run two passes closer together than this, whatever fired")
    ap.add_argument("--no-optimize", action="store_true",
                    help="legs and restarts only; never spend a session on the loop")
    ap.add_argument("--optimize-only", action="store_true",
                    help="run one pass now against the tree as it stands, and exit")
    opts, passthrough = ap.parse_known_args(argv)

    if opts.full_output:
        opts.max_result_lines = opts.max_input_lines = opts.max_line_chars = 0
    loop.enable_ansi()

    if not OPT_PROMPT.exists():
        say(f"missing {rel(OPT_PROMPT)}", C.RED)
        return 2

    if opts.optimize_only:
        state = read_json(OPTSTATE, default={}) or {}
        fired, ev = gather_signals(state)
        if not fired:
            say("no signal fired -- running the pass anyway, because you asked for it", C.YELLOW)
            fired = ["--optimize-only: run by hand, against whatever the evidence says"]
        say(run_pass(opts, fired, ev, int(state.get("since") or 0), verify_state()), C.CYAN)
        return 0

    if not opts.force and not claim():
        return 2
    try:
        return supervise(opts, passthrough)
    except KeyboardInterrupt:
        say("")
        say("interrupted -- the tree is consistent: every session commits before it exits, and "
            "a leg boundary is the only place this script writes anything", C.YELLOW)
        return 0
    finally:
        MARKER.unlink(missing_ok=True)
        loop.TICKER.stop()
        loop.CONSOLE.close()


if __name__ == "__main__":
    sys.exit(main())
