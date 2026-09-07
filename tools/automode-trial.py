#!/usr/bin/env python3
"""Whether the unattended loop should run under `--permission-mode auto` instead of
`bypassPermissions` -- measured once, against this repository, on this machine.

Bypass runs the loop with no permission check at all. Auto mode puts a model classifier in
front of every tool call the harness does not already consider safe, which buys a brake on
irreversible git and filesystem operations and a hard stop on egress, and costs a per-call
round trip plus the risk that the classifier is unreachable and holds a call back. Whether
that trade is worth taking is not a matter of opinion: it is a denial count, an overhead
percentage, and a commits-per-session comparison, and all three are measurable in one run.

So this script runs one short trial under auto mode, reads what happened out of three
sources, and prints a recommendation with the prompt that implements it.

    python tools/automode-trial.py                  # 3 sessions, compare against the last run
    python tools/automode-trial.py --sessions 5
    python tools/automode-trial.py --analyze 20260907-134014     # read a trial already run
    python tools/automode-trial.py --analyze 20260907-134014 --baseline 20260907-100657

What it reads:

  .loop/logs/<stamp>-NNNN.log   the session transcripts -- which shell calls were made, which
                                came back denied, and what the classifier said about each
  .automode_decisions.jsonl     the harness's own verdict log, enabled by AUTOMODE_DECISION_LOG=1.
                                One line per classifier call: decision, stage, severity, cost,
                                duration. This is the only place the *price* of auto mode is
                                recorded, and the harness deletes nothing, so the trial archives
                                it under .loop/automode/ and clears the root copy
  tools/loop-stats.py --json    what the trial's sessions cost and committed, and the same for a
                                baseline run, so "did the loop get less done" is a number

Nothing here decides anything a person cannot check: every threshold is a named constant at
the top of this file, and the recommendation prints the evidence that produced it.
"""

from __future__ import annotations

import argparse
import collections
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUNDIR = ROOT / ".loop"
LOGDIR = RUNDIR / "logs"
ARCHIVE = RUNDIR / "automode"
DECISIONS = ROOT / ".automode_decisions.jsonl"
SETTINGS = ROOT / ".claude" / "settings.json"

# ---------------------------------------------------------------------------------------------
# The thresholds. Every one of these is a judgement, not a measurement, which is why they are
# here rather than buried in the verdict function: disagree with one and edit it.

#: Classifier spend, as a share of the trial's total model spend, that still counts as cheap.
OVERHEAD_OK = 0.05
#: Wall clock the classifier may add, as a share of the trial's total, before it is worth
#: widening the allowlist rather than adopting as-is.
LATENCY_OK = 0.05
#: Commits per session may fall this far below the baseline before the trial counts as the loop
#: getting less done. Loose on purpose, and the number is measured rather than picked: two
#: consecutive *bypass* runs on 2026-09-07 came in at 1.25 and 1.80 commits per session, a 31%
#: spread with the permission mode held constant. Anything under that is the sample size talking.
COMMIT_DROP_OK = 0.40
#: and below this many sessions a side, the comparison is not made at all.
COMMIT_TEST_MIN_SESSIONS = 3
#: A single fail-closed hold (classifier unreachable, or its answer unparseable) is a fact about
#: an unattended run, not noise: nobody is there to retry it. More than this many and the
#: recommendation changes.
FAILCLOSED_OK = 0

#: Programs the harness treats as safe without asking the classifier, lifted from the CLI binary.
#: A call whose leading program is one of these never reaches the classifier and never costs
#: anything, which is why most of the loop's `grep`/`sed`/`head` traffic is already free.
SAFE_PROGRAMS = set("""ls cd cat rg grep find git gh node bun npm yarn pnpm cargo go make just
docker curl wget echo printf sed awk tr cut sort uniq xargs jq tee head tail wc which date diff
touch ln chmod mkdir cp mv rm ps kill pgrep pkill sleep stat env set export unset read source
command ssh scp tar zip unzip vim nano less more man tmux sudo bash sh zsh""".split())

#: The paths a denial on which means the loop cannot do its job at all, as opposed to a denial it
#: can route around. A block here is disqualifying however cheap the rest looked.
INFRA = ("tools/verify.py", "tools/session.py", "tools/splice.py", "git commit", "git add")

#: The tools that write the tree or commit it. Allowlisting these is the largest single saving
#: available and also the one that gives most of the safety back: they are where an unattended
#: session's mistakes become durable, and a rule for them means the classifier never sees a write
#: or a commit again. Proposed like anything else, because the saving is real -- but marked, so
#: the choice is made rather than defaulted into.
WRITE_PATHS = ("tools/splice.py", "tools/session.py")

DENIED = re.compile(r"denied by the Claude Code auto mode classifier", re.I)
REASON = re.compile(r"Reason:\s*([^.]{3,120})")
#: Fail-closed holds read as denials in the transcript but are not policy decisions -- the
#: classifier was unreachable, or its answer did not parse. Text-matched, so treat a zero here as
#: "none seen" rather than proof of none; the archived decision log is the authority.
FAILCLOSED = re.compile(r"classifier (?:was )?(?:unreachable|unavailable)|could not be parsed", re.I)
#: The shell operators a command is judged a segment at a time across.
OPERATORS = {"|", "||", "&&", ";", "&"}


def say(msg=""):
    print(msg, flush=True)


def rule(title):
    say()
    say(f"== {title}")


# ---------------------------------------------------------------------------------------------
# Reading what is on disk


def allow_rules():
    """The project's `permissions.allow`, which under auto mode is not a convenience but the
    thing that keeps a call away from the classifier entirely."""
    try:
        cfg = json.loads(SETTINGS.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return []
    return [r[5:-1] for r in cfg.get("permissions", {}).get("allow", []) if r.startswith("Bash(")]


def allowed_by(cmd, rules):
    for r in rules:
        if r.endswith(":*"):
            head = r[:-2]
            if cmd == head or cmd.startswith(head + " "):
                return True
        elif cmd.strip() == r:
            return True
    return False


def segments(cmd):
    """A command split at its shell operators, quoting respected. Splitting on a bare regex is
    what a first cut does and it is wrong here: `grep -n "pub fn|impl" src` is one command, and a
    regex splitter turns its alternation into a second segment whose program is `impl`, which
    then counts as something the classifier had to judge. `shlex` knows the difference. A command
    it cannot tokenise -- an unbalanced quote, a heredoc -- is returned whole, so it is judged as
    one unknown segment rather than silently dropped."""
    try:
        toks = shlex.split(cmd, posix=True)
    except ValueError:
        return [cmd]
    out, cur = [], []
    for t in toks:
        if t in OPERATORS:
            if cur:
                out.append(" ".join(cur))
            cur = []
        else:
            cur.append(t)
    if cur:
        out.append(" ".join(cur))
    return out or [cmd]


def free(cmd, rules):
    """True when every segment of this command is one the classifier never sees."""
    parts = [s for s in segments(cmd) if s.strip()]
    if not parts:
        return True
    for seg in parts:
        # Matched against the original spelling as well: `shlex` strips the quotes a permission
        # rule may have been written with.
        if allowed_by(seg, rules) or allowed_by(cmd, rules):
            continue
        prog = seg.split()[0].split("/")[-1].split("\\")[-1] if seg.split() else ""
        if prog in SAFE_PROGRAMS:
            continue
        return False
    return True


def transcripts(stamp):
    return sorted(p for p in LOGDIR.glob(f"{stamp}-*.log") if re.search(r"-\d{4}\.log$", p.name))


def read_run(stamp, rules):
    """Every shell call one run made, each tagged with whether it would have reached the
    classifier and whether it came back denied. The command is recovered by pairing each
    `tool_result` back to the `tool_use` that produced it, because the denial arrives on the
    result and the command is only ever on the call."""
    calls, denials, failclosed = [], [], []
    for path in transcripts(stamp):
        pending = {}
        for line in path.open(encoding="utf-8", errors="replace"):
            if not line.startswith("{"):
                continue
            try:
                ev = json.loads(line)
            except ValueError:
                continue
            if ev.get("type") == "assistant":
                for blk in ev.get("message", {}).get("content") or []:
                    if not isinstance(blk, dict) or blk.get("type") != "tool_use":
                        continue
                    inp = blk.get("input") or {}
                    cmd = " ".join(str(inp.get("command", "")).split())
                    pending[blk.get("id")] = (blk.get("name"), cmd)
                    if blk.get("name") in ("Bash", "PowerShell") and cmd:
                        calls.append({"session": path.name, "cmd": cmd,
                                      "classified": not free(cmd, rules)})
            elif ev.get("type") == "user":
                body = (ev.get("message") or {}).get("content")
                if not isinstance(body, list):
                    continue
                for blk in body:
                    if not isinstance(blk, dict) or blk.get("type") != "tool_result":
                        continue
                    text = blk.get("content")
                    text = text if isinstance(text, str) else json.dumps(text)
                    if not DENIED.search(text):
                        continue
                    tool, cmd = pending.get(blk.get("tool_use_id"), ("?", "(command not paired)"))
                    reason = REASON.search(text)
                    rec = {"session": path.name, "tool": tool, "cmd": cmd,
                           "reason": reason.group(1).strip() if reason else "(no reason given)"}
                    (failclosed if FAILCLOSED.search(text) else denials).append(rec)
    return calls, denials, failclosed


def read_decisions(path):
    out = []
    if not path.is_file():
        return out
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if line.startswith("{"):
            try:
                out.append(json.loads(line))
            except ValueError:
                pass
    return out


def loop_stats(stamp):
    cmd = [sys.executable, "tools/loop-stats.py", "--json"]
    if stamp:
        cmd += ["--run", stamp]
    try:
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                           encoding="utf-8", errors="replace", timeout=600)
        return json.loads(r.stdout) if r.returncode == 0 else None
    except (OSError, ValueError, subprocess.SubprocessError):
        return None


def run_stamps():
    """Every run stamp the log directory holds, oldest first."""
    seen = []
    for p in sorted(LOGDIR.glob("*-*.log")):
        m = re.match(r"(\d{8}-\d{6})-", p.name)
        if m and m.group(1) not in seen:
            seen.append(m.group(1))
    return seen


# ---------------------------------------------------------------------------------------------
# Running the trial


def preflight():
    problems = []
    if not LOGDIR.is_dir():
        problems.append(f"no {LOGDIR.relative_to(ROOT)} -- the loop has never run here")
    if (RUNDIR / "running").exists():
        problems.append(".loop/running says a driver is already up; stop it first")
    if not shutil.which("claude"):
        problems.append("`claude` is not on PATH")
    dirty = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, capture_output=True,
                           text=True, encoding="utf-8", errors="replace")
    if dirty.stdout.strip():
        problems.append("the tree is not clean -- a trial that mixes your edits into its "
                        "commits-per-session number measures nothing")
    return problems


def run_trial(opts):
    """One short run under auto mode, with the harness's verdict log switched on. The
    optimization pass is held out by default: it is the one session that rewrites the driver, so
    it is both the most likely to be blocked and the least like the sessions being measured."""
    if DECISIONS.exists():
        DECISIONS.unlink()
    before = set(run_stamps())
    cmd = [sys.executable, "tools/loop.py",
           "--permission-mode", "auto",
           "--max-sessions", str(opts.sessions),
           "--model", opts.model, "--no-hold"]
    if opts.effort:
        cmd += ["--effort", opts.effort]
    if not opts.keep_optimize:
        cmd.append("--no-optimize")
    env = dict(os.environ, AUTOMODE_DECISION_LOG="1")
    say(f"   {' '.join(cmd)}")
    say("   (AUTOMODE_DECISION_LOG=1)")
    say()
    subprocess.run(cmd, cwd=ROOT, env=env)

    stamp = None
    end = RUNDIR / "run-end.json"
    if end.is_file():
        try:
            stamp = json.loads(end.read_text(encoding="utf-8")).get("run_id")
        except (OSError, ValueError):
            stamp = None
    if stamp not in run_stamps():
        fresh = [s for s in run_stamps() if s not in before]
        stamp = fresh[-1] if fresh else None
    return stamp


def archive(stamp):
    """The verdict log lands in the working directory because that is where the harness puts it.
    Move it under `.loop/`, which is already ignored, so the trial leaves the tree as it found it."""
    if not DECISIONS.is_file():
        return None
    ARCHIVE.mkdir(parents=True, exist_ok=True)
    dest = ARCHIVE / f"{stamp or 'unstamped'}-decisions.jsonl"
    dest.write_bytes(DECISIONS.read_bytes())
    DECISIONS.unlink()
    return dest


# ---------------------------------------------------------------------------------------------
# The verdict


def proposals(calls, denied_cmds):
    """Allowlist entries that would take the most traffic off the classifier.

    Two rules keep this from proposing something it should not, and both matter more than the
    saving. First, only commands the classifier *allowed*: an entry proposed for a denied command
    converts a considered refusal into a standing exemption, which is the one thing an allowlist
    must never be used for. Second, only three shapes -- a repository tool, the built binary, a
    cargo subcommand. A first cut generalised whatever it saw and duly proposed `Bash(rm -rf:*)`,
    off a single `rm -rf` that reached it through a shell construct the splitter could not read.
    A blanket exemption for `rm -rf` is worse than every classifier call it would ever save, so
    the shapes are named here rather than inferred, and anything else is reported unproposed."""
    buckets, skipped = collections.Counter(), collections.Counter()
    for c in calls:
        if not c["classified"] or c["cmd"] in denied_cmds:
            continue
        toks = c["cmd"].split()
        if not toks:
            continue
        prog = toks[0].split("/")[-1].split("\\")[-1]
        if toks[0] in ("python", "python3") and len(toks) > 1 and toks[1].startswith("tools/") \
                and toks[1].endswith(".py"):
            buckets[f"{toks[0]} {toks[1]}:*"] += 1
        elif re.match(r"^\.?/?target/(debug|release)/[\w.-]+$", toks[0]):
            buckets[f"{toks[0]}:*"] += 1
        elif prog == "cargo" and len(toks) > 1 and toks[1].isalpha():
            buckets[f"cargo {toks[1]}:*"] += 1
        else:
            # Free already, or a shape no prefix rule expresses safely -- a heredoc, a `for`
            # loop, a destructive builtin. Counted so the report can say what it declined to
            # propose, never turned into a rule.
            skipped[" ".join(toks[:2])] += 1
    return buckets, skipped


def verdict(ev):
    """One of four answers, and the reasons that produced it."""
    why = []
    infra = [d for d in ev["denials"] if any(k in d["cmd"] for k in INFRA)]

    if not ev["sessions"]:
        return "INCONCLUSIVE", ["the trial produced no measurable session"]
    if not ev["decisions"]:
        # Without the verdict log there is no overhead measurement, and an "overhead was 0.0%"
        # read off an empty file is not a finding. This is also what analysing an old bypass run
        # looks like, which is a fair thing to ask of this script and a wrong thing for it to
        # answer ADOPT to.
        return "INCONCLUSIVE", [
            "no classifier verdicts were recorded, so nothing about auto mode was measured -- "
            "either this run was not under --permission-mode auto, or AUTOMODE_DECISION_LOG=1 "
            "was not set. Re-run `python tools/automode-trial.py` without --analyze."]
    if infra:
        why.append(f"{len(infra)} denial(s) landed on the loop's own machinery "
                   f"({', '.join(sorted({d['cmd'].split()[0] for d in infra}))}) -- "
                   "a session cannot verify, wrap or commit around that")
        return "STAY ON BYPASS", why
    if len(ev["failclosed"]) > FAILCLOSED_OK:
        why.append(f"{len(ev['failclosed'])} call(s) were held back fail-closed rather than "
                   "judged -- unattended, nobody is there to retry them")
        return "STAY ON BYPASS", why

    if ev["commit_drop"] is None:
        why.append("commits per session not compared -- too few sessions a side to mean anything")
    elif ev["commit_drop"] > COMMIT_DROP_OK:
        why.append(f"commits per session fell {ev['commit_drop']:.0%} against the baseline "
                   f"({ev['commits_trial']:.2f} vs {ev['commits_base']:.2f}) -- the loop got "
                   "less done, whatever it cost")
        return "STAY ON BYPASS", why

    if ev["denials"]:
        why.append(f"{len(ev['denials'])} denial(s), none on the loop's machinery -- each is a "
                   "call the classifier judged and refused, so read them before allowlisting")
    else:
        why.append("no denials: every call the classifier saw, it allowed")
    if ev["cost_share"] > OVERHEAD_OK or ev["time_share"] > LATENCY_OK:
        why.append(f"classifier overhead is {ev['cost_share']:.1%} of spend and "
                   f"{ev['time_share']:.1%} of wall clock -- above the {OVERHEAD_OK:.0%} bar, so "
                   "widen the allowlist before adopting")
        return "ADOPT WITH RULES", why
    why.append(f"classifier overhead is {ev['cost_share']:.1%} of spend and "
               f"{ev['time_share']:.1%} of wall clock")
    return ("ADOPT WITH RULES" if ev["denials"] else "ADOPT"), why


def implementation_prompt(answer, ev):
    """The paste-ready next step. It names files and line anchors rather than describing them,
    because the session that runs it should not have to re-derive what this script already knows."""
    entries = [f'      "Bash({e})",' for e, _ in ev["proposals"].most_common(12)]
    block = "\n".join(entries) if entries else "      (nothing worth adding -- the allowlist already covers it)"

    if answer == "INCONCLUSIVE":
        return ("Nothing to implement yet -- the trial did not measure auto mode.\n\n"
                + "".join(f"  - {w}\n" for w in ev["why"])
                + "\nRun `python tools/automode-trial.py --sessions 3` against a clean tree, "
                  "then paste the prompt it prints.")

    if answer == "STAY ON BYPASS":
        return (
            "Keep the unattended loop on bypassPermissions, and record why.\n\n"
            "Evidence, from `python tools/automode-trial.py`:\n"
            + "".join(f"  - {w}\n" for w in ev["why"])
            + "\nDo this:\n"
            "1. Add a playbook bullet to docs/agent/playbook.md recording that auto mode was "
            "trialled and what blocked it, with an `[until:]` trailer naming the condition that "
            "would make it worth retrying (docs/agent/conventions.md - A playbook bullet).\n"
            "2. Leave tools/loop.py's --permission-mode default alone.\n"
            "3. Do not widen .claude/settings.json for this; the allowlist is not the blocker.\n\n"
            "Verify with `python tools/verify.py`, then wrap with `python tools/session.py --wrap`."
        )

    steps = [
        "1. In tools/loop.py, change the `--permission-mode` argparse default from "
        '"bypassPermissions" to "auto". Update the flag\'s help and the module docstring to say '
        "the loop runs classified by default and why.",
        "2. Keep the optimization pass on bypass: it rewrites tools/loop.py and "
        "docs/agent/session-prompt.md, which the classifier reads as self-modification. Give "
        "`run_optimization` its own mode rather than reusing `opts.permission_mode`.",
    ]
    if entries:
        writes = [e for e, _ in ev["proposals"].most_common(12)
                  if any(w in e for w in WRITE_PATHS)]
        note = ""
        if writes:
            note = ("\n   Decide these two separately -- " + ", ".join(writes) + " write the tree "
                    "or commit it, so a rule for them means no write and no commit is ever "
                    "classified again. That is the largest saving on the list and most of the "
                    "safety on it. Say in the commit message which way you went and why.")
        steps.append(
            "3. Add these entries to .claude/settings.json under permissions.allow -- each was "
            "measured in the trial, and every one is a command the classifier saw and allowed:\n"
            + block + note
        )
    steps.append(
        f"{len(steps) + 1}. Record the measured overhead "
        f"({ev['cost_share']:.1%} of spend, {ev['time_share']:.1%} of wall clock, "
        f"{ev['classified']} classified calls over {ev['sessions']} sessions) in the ADR or "
        "playbook bullet that lands the change, so the next person does not re-measure it."
    )
    if ev["denials"]:
        steps.append(
            f"{len(steps) + 1}. Read these {len(ev['denials'])} denial(s) before deciding whether "
            "they need a rule or are correct as they stand -- do NOT allowlist them blindly:\n"
            + "".join(f"      - {d['cmd'][:100]}  ({d['reason']})\n" for d in ev["denials"][:10])
        )

    return ("Switch the unattended loop to auto mode.\n\nEvidence, from "
            "`python tools/automode-trial.py`:\n"
            + "".join(f"  - {w}\n" for w in ev["why"])
            + "\nDo this:\n" + "\n".join(steps)
            + "\n\nVerify with `python tools/verify.py`, then wrap with "
              "`python tools/session.py --wrap`.")


# ---------------------------------------------------------------------------------------------


def analyse(stamp, baseline, decisions_path):
    rules = allow_rules()
    calls, denials, failclosed = read_run(stamp, rules)
    decisions = read_decisions(decisions_path) if decisions_path else []

    trial = loop_stats(stamp) or {"sessions": [], "constants": {}}
    base = loop_stats(baseline) if baseline else None

    sessions = trial["sessions"]
    cost = sum(s.get("cost_usd") or 0 for s in sessions)
    wall = sum(s.get("duration_ms") or 0 for s in sessions) / 1000.0
    cls_cost = sum(d.get("costUSD") or 0 for d in decisions)
    cls_time = sum(d.get("durationMs") or 0 for d in decisions) / 1000.0

    commits_trial = (sum(s.get("commits") or 0 for s in sessions) / len(sessions)) if sessions else 0
    commits_base = None
    if base and base["sessions"]:
        commits_base = sum(s.get("commits") or 0 for s in base["sessions"]) / len(base["sessions"])
    drop = None
    if commits_base and len(sessions) >= COMMIT_TEST_MIN_SESSIONS \
            and len(base["sessions"]) >= COMMIT_TEST_MIN_SESSIONS:
        drop = max(0.0, (commits_base - commits_trial) / commits_base)

    denied_cmds = {d["cmd"] for d in denials}
    proposed, unproposed = proposals(calls, denied_cmds)
    ev = {
        "stamp": stamp, "baseline": baseline,
        "sessions": len(sessions), "calls": len(calls),
        "classified": sum(1 for c in calls if c["classified"]),
        "decisions": decisions, "denials": denials, "failclosed": failclosed,
        "cost": cost, "wall": wall, "cls_cost": cls_cost, "cls_time": cls_time,
        "cost_share": (cls_cost / cost) if cost else 0.0,
        "time_share": (cls_time / wall) if wall else 0.0,
        "commits_trial": commits_trial, "commits_base": commits_base, "commit_drop": drop,
        "proposals": proposed, "unproposed": unproposed,
    }

    rule(f"TRIAL {stamp}" + (f"   baseline {baseline}" if baseline else "   (no baseline)"))
    say(f"   sessions            {ev['sessions']}")
    say(f"   shell calls         {ev['calls']}")
    say(f"   reached classifier  {ev['classified']}"
        f"   ({ev['classified'] / ev['calls']:.0%} of them)" if ev["calls"] else "")
    say(f"   verdicts logged     {len(decisions)}"
        + ("" if decisions else "   -- no decision log; was AUTOMODE_DECISION_LOG=1 set?"))
    if decisions:
        by = collections.Counter(d.get("decision", "?") for d in decisions)
        say(f"   verdicts            {', '.join(f'{k} {v}' for k, v in by.most_common())}")
        sev = [d.get("stage1Severity") for d in decisions if d.get("stage1Severity") is not None]
        if sev:
            say(f"   severity            min {min(sev)}, max {max(sev)}, "
                f"mean {sum(sev) / len(sev):.1f}")

    rule("COST")
    say(f"   trial spend         ${ev['cost']:.2f} over {ev['wall'] / 60:.0f} min")
    say(f"   classifier          ${ev['cls_cost']:.2f} ({ev['cost_share']:.1%}), "
        f"{ev['cls_time'] / 60:.1f} min ({ev['time_share']:.1%})")
    if commits_base is not None:
        say(f"   commits/session     {commits_trial:.2f} trial vs {commits_base:.2f} baseline")
    else:
        say(f"   commits/session     {commits_trial:.2f} (no baseline to compare against)")

    rule("DENIALS")
    if not denials and not failclosed:
        say("   none -- every call the classifier saw, it allowed")
    for d in denials:
        say(f"   [denied]      {d['cmd'][:110]}")
        say(f"                 {d['reason']}  ({d['session']})")
    for d in failclosed:
        say(f"   [fail-closed] {d['cmd'][:110]}   ({d['session']})")

    answer, why = verdict(ev)
    ev["why"] = why

    rule("RECOMMENDATION")
    say(f"   {answer}")
    for w in why:
        say(f"     - {w}")

    if ev["proposals"]:
        rule("ALLOWLIST ENTRIES THIS TRIAL WOULD JUSTIFY")
        for entry, n in ev["proposals"].most_common(12):
            mark = "   <- writes or commits; allowlisting it retires the classifier from that " \
                   "path" if any(w in entry for w in WRITE_PATHS) else ""
            say(f"   {n:5d}  Bash({entry}){mark}")
    if ev["unproposed"]:
        rule("REACHED THE CLASSIFIER, NOT PROPOSED")
        say("   No prefix rule expresses these safely -- heredocs, shell loops, destructive")
        say("   builtins. They keep paying a classifier call, which is the correct outcome.")
        for entry, n in ev["unproposed"].most_common(10):
            say(f"   {n:5d}  {entry}")

    rule("PROMPT -- paste this into a fresh session to implement the recommendation")
    say()
    print(implementation_prompt(answer, ev))
    say()
    return 0 if answer.startswith("ADOPT") else 1


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0],
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sessions", type=int, default=3,
                    help="sessions in the trial run (default 3)")
    ap.add_argument("--model", default="opus")
    ap.add_argument("--effort", default=None,
                    choices=("low", "medium", "high", "xhigh", "max"))
    ap.add_argument("--baseline", metavar="STAMP",
                    help="run stamp to compare against; default is the run before the trial")
    ap.add_argument("--analyze", metavar="STAMP",
                    help="skip the run and analyse a trial already on disk")
    ap.add_argument("--keep-optimize", action="store_true",
                    help="let the trial run an optimization pass. Off by default: that pass "
                         "rewrites the driver, which is the one thing auto mode is most likely "
                         "to block, and it is not the session being measured")
    opts = ap.parse_args()

    if opts.analyze:
        stamp = opts.analyze
        if stamp not in run_stamps():
            sys.exit(f"no logs for run {stamp} under {LOGDIR.relative_to(ROOT)}")
        baseline = opts.baseline or next(
            (s for s in reversed(run_stamps()) if s != stamp), None)
        archived = ARCHIVE / f"{stamp}-decisions.jsonl"
        return analyse(stamp, baseline, archived if archived.is_file() else None)

    problems = preflight()
    if problems:
        say("preflight failed:")
        for p in problems:
            say(f"   - {p}")
        return 2

    known = run_stamps()
    baseline = opts.baseline or (known[-1] if known else None)

    rule(f"RUNNING {opts.sessions} SESSION(S) UNDER --permission-mode auto")
    stamp = run_trial(opts)
    if not stamp:
        say("could not identify the trial's run stamp; nothing to analyse")
        return 2
    archived = archive(stamp)
    if archived:
        say(f"\n   decision log archived to {archived.relative_to(ROOT)}")
    return analyse(stamp, baseline, archived)


if __name__ == "__main__":
    sys.exit(main())
