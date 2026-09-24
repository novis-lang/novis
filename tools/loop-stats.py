#!/usr/bin/env python3
"""What the last run's sessions actually cost, measured out of `.loop/logs/*.log`.

Every number the loop's design rests on -- how long a session takes, how much of it is fixed
cost, how fast context grows, how many slices fit before compaction -- is a *measurement of
this repository on this machine with this model*, not a constant. All three change. So this
script re-derives them from the NDJSON transcripts rather than restating them, and
`docs/agent/loop-authoring.md` § *Measure first* makes running it the first step of setting
any new goal.

It stores no facts and enforces nothing. It reads the transcripts, prints what it found, and
projects the group size those findings imply. If the projection disagrees with what the
session prompt currently says, the prompt is the thing that is stale.

What it measures, per session:

  calls           tool calls -- the unit a session's wall clock is proportional to
  calls/msg       tool calls per assistant message; 1.00 means no message carried two
  cmd/call        commands per shell call -- a `;`/`&&` chain is batching too, and it is the
                  only kind that has ever happened here. Read the two together: `calls/msg`
                  1.00 beside `cmd/call` 1.97 is a session batching inside the shell, which
                  is what `peek.py` and the chaining habit ask for, not a saving left on
                  the table.
  head            calls before the first edit: orientation, paid once per session
  work            calls between the first edit and the first verify: the part that ships
  tail            calls from the first verify on: verify, docs, handoff, commit -- also once
  ctx             context at the first and last assistant message, and the slope between
  compaction      a context drop, which is the failure a group must stay under

`--json` prints the same thing as one object, for a session that wants to read it rather
than a human. `--run <stamp>` limits it to one run's logs.
"""

from __future__ import annotations

import argparse
import json
import re
import shlex
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOGDIR = ROOT / ".loop" / "logs"

# A tail begins at the first verification call: everything from there on is verify, docs,
# handoff and commit, which a group pays once no matter how many slices sit in front of it.
# `verify.py` stays in the list because the logs of every run before `nv verify` name it.
VERIFY_MARKERS = ("nv verify", "verify.py", "tools/verify")
MUTATORS = ("Edit", "Write", "NotebookEdit")

#: A top-level command separator inside one shell call. `|` is deliberately not one -- a pipeline
#: is a single command -- and the lookarounds keep `||` from being counted as two.
SEPARATOR_RE = re.compile(r"(?<!\|)(?:;|&&)(?!\|)")

#: A shell call carrying file content, which AGENTS.md rule 1 forbids outright: the shell parses
#: apostrophes and backticks before it runs anything, so the tree gets whatever survived that.
#: The three spellings the rule names, and nothing else --
#:
#:   * a heredoc, `<<'TAG'` / `<<"TAG"` / `<<TAG`, including one writing a script under
#:     `.agent-tmp/` that then patches the tree, which is the shape this has actually taken;
#:   * `sed -i`;
#:   * a `>`/`>>` redirect to a path. `2>&1` and `>&2` are excluded by the lookbehind and the
#:     `&` guard -- they redirect a stream, not content -- and so is `/dev/null`.
#:
#: Counted because it was invisible: nothing reported it, and a session doing it read as one that
#: made no edits at all, since none of `MUTATORS` fired. `bun nv splice --patch` is the tool that
#: replaces all three, and it is one call for any number of files.
#: The text of a Bash call that applied a splice patch: the `nv` command, and the Python tool it
#: replaced, which the logs of older runs name.
SPLICE_MARKERS = ("nv splice", "splice.py")

SHELL_WRITE_RE = re.compile(
    r"<<-?\s*['\"]?\w+"
    r"|\bsed\s+(?:[^|;&]*\s)?-i\b"
    r"|(?<![0-9&<>])>>?\s*(?!&|/dev/null)[^\s|;&<>]+"
)


def shell_write(name, inp) -> bool:
    """Does this call carry file content through a shell? See [`SHELL_WRITE_RE`]."""
    if name not in ("Bash", "PowerShell") or not isinstance(inp, dict):
        return False
    return bool(SHELL_WRITE_RE.search(str(inp.get("command") or "")))

# The ceiling is a QUALITY limit, not a capacity one, and it is deliberately a fixed number
# rather than whatever the model reports. A coding agent degrades noticeably long before its
# window is full -- it starts missing things it has already read -- and a session that degrades
# produces work the acceptance test then rejects, which costs far more than the session saved.
# 200k is the safe zone; the measured window (1M, at the time of writing) is read from the
# transcripts anyway, but only to catch the case where it is *smaller* than this and capacity
# binds first. Raise this only with evidence, and re-run every projection below when you do.
CONTEXT_CEILING = 200_000


def call_text(call):
    """The searchable text of one tool call: its command, its path, or its whole input."""
    name, inp = call
    if not isinstance(inp, dict):
        return str(inp)
    return str(inp.get("command") or inp.get("file_path") or json.dumps(inp))


# ------------------------------------------------------------------------- attribution
#
# Where a session's context actually went, by what put it there. This is the measurement the
# `[context]` manifest in loop-goal.toml is written against: a goal whose sessions spend a
# third of their budget reading whole ADRs is a goal whose manifest should be naming ADR
# *sections*, and the only way to know that is to charge every byte to the call that fetched
# it. Buckets are matched in order, first hit wins, against one TARGET at a time: a batched
# `peek.py` call is split into the paths it names (`targets_of`), because matched as one string
# the first orientation path in it charged every source file beside it to `orientation` -- so the
# better a session followed AGENTS.md rule 2, the more it read as re-reading the pack.
#
# `orientation` names the files the pack is built from, by full path, and nothing else. A bare
# `docs/agent/` or `playbook` matched `tools/playbook.py`'s source and every process doc AGENTS.md
# routes to, none of which the pack carries. The goal and its acceptance list get their own bucket:
# the pack carries one item of the list and none of the rest, so a session reading them is usually
# fetching what the pack left out -- the opposite defect from re-reading what it held.

BUCKETS = (
    ("goal", ("docs/agent/goals/", "loop-goal")),
    ("orientation", ("orient.py", "brief.py", "docs/agent/handoff.md", "docs/agent/playbook",
                     "docs/agent/conventions.md", "AGENTS.md", "CLAUDE.md")),
    ("process docs", ("docs/agent/",)),
    ("adr", ("docs/adr/", "docs/decisions/")),
    ("plan + spec", ("implementation-plan", "docs/plan/", "docs/spec/", "plan.py")),
    ("build + test", ("nv verify", "verify.py", "cargo ", "nvs test", "nvs run", "loop.py")),
    ("git", ("git ",)),
    ("discovery", ("grep", "rg ", "find ", " ls ", "glob", "Glob", "Grep")),
    ("source", ("crates/", "benches/", "tests/", "examples/", "tools/", "fuzz/")),
)


def bucket_of(name, text):
    if name in MUTATORS:
        return "writing"
    # A Windows path is spelled with backslashes -- doubled once `call_text` has JSON-dumped it --
    # and every needle above is spelled with `/`.
    text = re.sub(r"\\+", "/", text)
    for label, needles in BUCKETS:
        if any(nd in text for nd in needles):
            return label
    return "other"


#: A shell call running `peek.py`, whose arguments are the paths it reads.
PEEK_RE = re.compile(r"\bpeek\.py\b")

#: Where one `peek.py` target's output begins. Every target opens with this line naming itself.
PEEK_SECTION_RE = re.compile(r"^===== (.*)$", re.M)


def is_peek(name, inp) -> bool:
    return name in ("Bash", "PowerShell") and bool(PEEK_RE.search(call_text((name, inp))))


def targets_of(name, inp):
    """What one call reads, as the strings `bucket_of` should see: the paths a `peek.py` call names,
    or the call's whole text for anything else. `--locate` takes symbols rather than paths, and a
    `;`/`&&` chain puts other commands' words in the list, so both stay one target. Split without
    POSIX escapes, because a PowerShell path's backslashes are separators, not escapes."""
    text = call_text((name, inp))
    m = PEEK_RE.search(text) if is_peek(name, inp) else None
    if not m or SEPARATOR_RE.search(text):
        return [text]
    try:
        words = shlex.split(text[m.end():].split("|", 1)[0], posix=False)
    except ValueError:
        return [text]
    if "--locate" in words:
        return [text]
    paths = [w for w in words if not w.startswith("-")]
    return paths or [text]


def shares_of(name, inp):
    """One call's buckets, each with the share of the call it takes: `{label: fraction}`."""
    targets = targets_of(name, inp)
    shares = {}
    for t in targets:
        label = bucket_of(name, t)
        shares[label] = shares.get(label, 0) + 1 / len(targets)
    return shares


def charge_result(attribution, shares, peek, body):
    """Charge one tool result's bytes to the buckets of the call that fetched it.

    A result is sized JSON-dumped when it is not a plain string. A `peek.py` result opens every
    target with a `===== <target>` line, so a batched read is charged section by section to the
    bucket its own header names -- the handoff's lines to `orientation`, the source file's beside
    it to `source` -- and only what sits outside every section is split by the call's shares."""
    size = len(body if isinstance(body, str) else json.dumps(body))
    charged = 0
    if peek:
        if isinstance(body, str):
            plain = body
        elif isinstance(body, list):
            plain = "".join(b.get("text", "") for b in body if isinstance(b, dict))
        else:
            plain = ""
        found = list(PEEK_SECTION_RE.finditer(plain))
        for m, end in zip(found, [n.start() for n in found[1:]] + [len(plain)]):
            label = bucket_of(None, m.group(1))
            attribution[label] = attribution.get(label, 0) + (end - m.start())
            charged += end - m.start()
    rest = max(size - charged, 0)
    for label, share in shares.items():
        attribution[label] = attribution.get(label, 0) + round(rest * share)


def subagent_cost(path):
    """The sessions this transcript delegated to, out of `.loop/logs/<stem>.subagents/`.

    A subagent's turns are inlined in the parent's stream and `read_session` drops them there,
    so without this a delegated read is a session that mysteriously did a great deal with very
    few calls. `tools/loop.py` copies the harness's own transcripts there; if the directory is
    absent, nothing was delegated -- or the run predates that capture, which is why the count is
    reported separately rather than folded into the session's own figures."""
    directory = path.with_suffix(".subagents")
    if not directory.is_dir():
        return []
    found = []
    for agent in sorted(directory.glob("*.jsonl")):
        calls, peak = 0, 0
        for line in agent.read_text(encoding="utf-8", errors="replace").splitlines():
            if not line.strip():
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("type") != "assistant":
                continue
            message = event.get("message", {})
            calls += sum(1 for c in message.get("content", []) or []
                         if isinstance(c, dict) and c.get("type") == "tool_use")
            usage = message.get("usage", {})
            peak = max(peak, sum(usage.get(k, 0) for k in
                                 ("input_tokens", "cache_creation_input_tokens",
                                  "cache_read_input_tokens")))
        if calls:
            found.append({"agent": agent.stem, "calls": calls, "peak_ctx": peak})
    return found


def read_session(path):
    """One transcript -> the measurements above, or None if it holds no assistant turn."""
    calls, contexts, per_message, result = [], [], [], None
    shell_calls, shell_cmds = 0, 0
    names, attribution = {}, {}
    pack_bytes, pack_goal = 0, ""
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue  # a truncated final line is normal for a killed run
        # A subagent's turns are inlined here, tagged with the tool call that spawned them, and
        # they are the parent's context only in the sense that it waited for them: its own window
        # holds the Agent tool's one result. Counted, they charge the parent for calls it never
        # made and put a 124k -> 12k -> 129k step in its context series, which `drops` below reads
        # as a compaction -- and a compaction is what loop-authoring.md's table turns into "the
        # ceiling is far too high". `subagent_cost` prices them from `<stem>.subagents/` instead.
        if event.get("parent_tool_use_id"):
            continue
        kind = event.get("type")
        if kind == "loop_pack":
            pack_bytes = event.get("bytes") or 0
            pack_goal = event.get("goal") or ""
            continue
        if kind == "result":
            result = event
            continue
        if kind == "user":
            # A tool result is the single largest thing that enters a conversation, and it is
            # charged to the call that asked for it -- which is why the ids are kept above.
            for block in event.get("message", {}).get("content", []) or []:
                if not isinstance(block, dict) or block.get("type") != "tool_result":
                    continue
                shares, peek = names.get(str(block.get("tool_use_id")), ({"other": 1.0}, False))
                charge_result(attribution, shares, peek, block.get("content"))
            continue
        if kind != "assistant":
            continue
        message = event.get("message", {})
        content = message.get("content", [])
        tool_uses = [c for c in content if c.get("type") == "tool_use"]
        if tool_uses:
            per_message.append(len(tool_uses))
            calls.extend((c.get("name"), c.get("input")) for c in tool_uses)
            for c in tool_uses:
                names[str(c.get("id"))] = (shares_of(c.get("name"), c.get("input")),
                                           is_peek(c.get("name"), c.get("input")))
                # A `;`/`&&` chain is a batch: one round trip, several commands. Counting only
                # calls-per-message called a session that chained 39 commands into one call
                # "never batched", which is the opposite of what it did. Pipes are one command.
                inp = c.get("input")
                cmd = inp.get("command") if isinstance(inp, dict) else None
                if c.get("name") in ("Bash", "PowerShell") and cmd:
                    shell_calls += 1
                    shell_cmds += len(SEPARATOR_RE.findall(cmd)) + 1
                # An Edit's *input* is the new code, which is real context the session spent.
                if c.get("name") in MUTATORS:
                    attribution["writing"] = attribution.get("writing", 0) + len(
                        json.dumps(c.get("input") or {})
                    )
        usage = message.get("usage", {})
        total = sum(
            usage.get(k, 0)
            for k in ("input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens")
        )
        if total:
            contexts.append(total)

    if not calls:
        return None

    texts = [call_text(c) for c in calls]
    # `nv splice` and a shell write are edits too. Leaving them out put a session that patched
    # the tree entirely through heredocs -- 0053 of the 20260828-112939 run -- at 52 head calls
    # and no work at all, which is not a slow orientation but a mis-read one. `SPLICE_MARKERS`
    # carries `splice.py` as well, because the logs of every run before `nv splice` name it.
    shell_writes = [i for i, (name, inp) in enumerate(calls) if shell_write(name, inp)]
    mutations = sorted(
        [i for i, (name, _) in enumerate(calls) if name in MUTATORS]
        + shell_writes
        + [i for i, t in enumerate(texts) if any(m in t for m in SPLICE_MARKERS)]
    )
    verifies = [i for i, t in enumerate(texts) if any(m in t for m in VERIFY_MARKERS)]
    # One VERIFICATION, not one call about one. `--start` and the `--wait` that collects it are
    # two calls over a single run of the seven steps, so counting calls reported a session that
    # verified twice as having verified four times -- against a rule that says "once, at the end".
    runs = [i for i in verifies if "--wait" not in texts[i]]
    # A session's commits go through `session.py --wrap`, which is the ONLY spelling that commits
    # -- so counting `git commit` counted the hand-rolled git the wrap exists to remove, and read
    # 0 for 33 of 39 sessions of one run while the ledger showed 2-6 each. Count the wrap, and the
    # hand-rolled one beside it, and the column means "this session committed" again.
    commits = [i for i, t in enumerate(texts) if "git commit" in t or "session.py --wrap" in t]

    n = len(calls)
    head = mutations[0] if mutations else n
    # The tail begins after the LAST mutation, not at the first verification.
    #
    # `verify.py --start` is the recommended shape -- its own docstring says "start the run, write
    # the wrap, then collect" -- and it is fired mid-work, so `verifies[0]` lands in the middle of
    # a session rather than at the end of it. Measured across one 39-session run, the span from
    # `verifies[0]` onwards held 103 Edit and 68 Write calls: that is work being counted as fixed
    # cost. The number that came out, "fixed cost 31 of 67 calls (46%)", is quoted in AGENTS.md
    # § *Session workflow* as the reason the group gate is a context budget, and it was inflated
    # by the very habit this file recommends.
    #
    # After the last mutation is the honest boundary: everything from there is collecting a
    # verification and applying the wrap, which is what "tail" was always meant to name.
    tail_start = (mutations[-1] + 1) if mutations else (verifies[0] if verifies else n)
    # A session that verified before it edited anything did something unusual; fold the
    # oddity into `work` rather than reporting a negative phase.
    tail_start = max(tail_start, head)

    # The head is meant to be near-zero in the loop: the driver runs `orient.py` and pipes the
    # pack in ahead of the prompt, so a session opens already oriented and its first call could
    # be an edit. It is not -- the head runs to a third of a session. WHICH bucket those calls
    # fall in is the whole question, because the two explanations need opposite fixes: a pack
    # missing something the goal needs is a `[context]` manifest to widen, while a session
    # re-reading what the pack already said is a pack to make more legible. A single number
    # cannot tell those apart, so keep the breakdown rather than the count.
    #
    # A batched call is split across the buckets of the targets it names, so a head count is a sum
    # of fractions: a `peek.py` of the handoff and one source file is half a re-read.
    head_buckets = {}
    for i in range(head):
        for label, share in shares_of(*calls[i]).items():
            head_buckets[label] = head_buckets.get(label, 0) + share

    drops = [
        (i, contexts[i - 1], contexts[i])
        for i in range(1, len(contexts))
        if contexts[i] < contexts[i - 1] * 0.6
    ]

    return {
        "log": path.name,
        "calls": n,
        "per_message": round(sum(per_message) / len(per_message), 2) if per_message else 0.0,
        "per_shell_call": round(shell_cmds / shell_calls, 2) if shell_calls else 0.0,
        "head": head,
        "head_buckets": head_buckets,
        "work": tail_start - head,
        "tail": n - tail_start,
        "verify_runs": len(runs),
        "shell_writes": len(shell_writes),
        "commits": len(commits),
        "ctx_start": contexts[0] if contexts else 0,
        "ctx_end": max(contexts) if contexts else 0,
        "pack_bytes": pack_bytes,
        "pack_goal": pack_goal,
        "compactions": len(drops),
        "duration_ms": (result or {}).get("duration_ms"),
        "duration_api_ms": (result or {}).get("duration_api_ms"),
        "cost_usd": (result or {}).get("total_cost_usd"),
        "model_usage": (result or {}).get("modelUsage"),
        "complete": result is not None,
        "attribution": attribution,
        "subagents": subagent_cost(path),
    }


def context_window(sessions):
    """The window the model actually had, read out of the transcript rather than assumed.

    A `result` event carries `modelUsage[<model>].contextWindow`, so this is a measurement
    like every other number here. The first version of this script guessed 400k and was
    wrong by two and a half times, which changed the recommended group size from 4 to 2 --
    the exact failure the *Measure first* rule exists to prevent.
    """
    best = 0
    for s in sessions:
        for usage in (s.get("model_usage") or {}).values():
            if not isinstance(usage, dict):
                continue
            # Ignore the small side-model a session uses for summaries and titles. Almost
            # all of a session's input is cache reads, so `inputTokens` alone is ~nothing.
            seen = sum(
                usage.get(k, 0)
                for k in ("inputTokens", "cacheReadInputTokens", "cacheCreationInputTokens")
            )
            if seen < 100_000:
                continue
            best = max(best, usage.get("contextWindow", 0))
    return best or None


def totals(sessions):
    """The constants the projection needs, averaged over whole sessions only."""
    whole = [s for s in sessions if s["complete"] and s["duration_ms"]]
    if not whole:
        return None
    calls = sum(s["calls"] for s in whole)
    return {
        "sessions": len(whole),
        "seconds_per_call": sum(s["duration_ms"] for s in whole) / 1000 / calls,
        "api_share": sum(s["duration_api_ms"] or 0 for s in whole)
        / sum(s["duration_ms"] for s in whole),
        "ctx_per_call": sum(s["ctx_end"] - s["ctx_start"] for s in whole) / calls,
        "ctx_start": sum(s["ctx_start"] for s in whole) / len(whole),
        "head": sum(s["head"] for s in whole) / len(whole),
        "work": sum(s["work"] for s in whole) / len(whole),
        "tail": sum(s["tail"] for s in whole) / len(whole),
        "cost_per_session": sum(s["cost_usd"] or 0 for s in whole) / len(whole),
    }


def live_ctx_start(sessions):
    """What the NEXT session will open at, rather than what the last ones opened at.

    The projection decides a slice cap, and a cap is advice about the run you are about to do.
    `ctx_start` averaged over the transcripts answers a different question: it is the mean of
    every pack the last run happened to carry, and the pack is the one part of the floor anybody
    ever changes. A pass that halves it would otherwise go on producing the old cap until a whole
    run had been spent re-measuring the thing that was just measured.

    So: the regressed fixed floor -- harness prompt, tool schemas, CLAUDE.md, AGENTS.md, none of
    which moves -- plus the pack that is on disk right now, at the regressed bytes-per-token.
    Falls back to the historical mean when there is no calibration to regress from, because a
    projection from a guessed constant is worse than one from a stale measurement."""
    points = [(s["pack_bytes"], s["ctx_start"]) for s in sessions
              if s.get("pack_bytes") and s.get("ctx_start")]
    if len({p for p, _ in points}) < 2:
        return None
    n = len(points)
    mx = sum(p for p, _ in points) / n
    my = sum(c for _, c in points) / n
    sxx = sum((p - mx) ** 2 for p, _ in points)
    if sxx == 0:
        return None
    slope = sum((p - mx) * (c - my) for p, c in points) / sxx
    intercept = my - slope * mx
    try:
        # Captured as **bytes**, the way `session.py` measures this same pack and for the reason
        # written down there: under `text=True` Python decodes the pipe with the console's own
        # codepage, which on this box is cp1252, so the pack's `§` raises `UnicodeDecodeError`
        # inside the reader thread. `proc.stdout` then comes back `None` and the length below
        # raises `AttributeError` — which is not a `SubprocessError` and so escapes the `except`,
        # taking the projection table and the caps with it, the two sections an optimization pass
        # reads to decide whether the ceiling moves. Bytes are what the regression wants anyway,
        # and they are the same count `session.py` recorded, so there is nothing to decode.
        proc = subprocess.run(
            [sys.executable, str(ROOT / "tools" / "orient.py")],
            capture_output=True, cwd=ROOT, timeout=60,
        )
        if proc.returncode != 0 or not proc.stdout:
            return None
        now = len(proc.stdout)
    except (OSError, subprocess.SubprocessError):
        return None
    return {"tokens": intercept + slope * now, "pack": now, "floor": intercept,
            "per_byte": slope}


def project(t, ceiling):
    """For a group of N slices: wall clock and token cost against N separate sessions.

    Tokens are dominated by cache reads, and a turn re-reads the whole context, so a
    session's token bill goes as calls x mean context -- quadratic in its own length. That
    is what puts a ceiling on a group: past some N a session is still faster but no longer
    cheaper, and past a further N it compacts and loses the standing instructions.
    """
    rows = []
    for n in range(1, 9):
        calls = t["head"] + t["tail"] + n * t["work"]
        ctx_end = t["ctx_start"] + t["ctx_per_call"] * calls
        tokens = calls * (t["ctx_start"] + ctx_end) / 2

        solo_calls = t["head"] + t["tail"] + t["work"]
        solo_ctx_end = t["ctx_start"] + t["ctx_per_call"] * solo_calls
        solo_tokens = n * solo_calls * (t["ctx_start"] + solo_ctx_end) / 2

        rows.append(
            {
                "slices": n,
                "calls": round(calls),
                "minutes": calls * t["seconds_per_call"] / 60,
                "speedup": (n * solo_calls) / calls,
                "token_ratio": tokens / solo_tokens,
                "ctx_end": round(ctx_end),
                "over_ceiling": ctx_end > ceiling,
            }
        )
    return rows


def render_attribution(sessions):
    """Where the context went, and therefore what the next goal's manifest should narrow."""
    print("\n== WHERE THE CONTEXT WENT  (bytes charged to the call that fetched them)")
    print("-- approximate tokens: bytes / 3.6. Read this as shares, not as absolutes.")
    print("-- a session's OTHER fixed cost -- the harness prompt, the tool schemas, CLAUDE.md")
    print("   and AGENTS.md -- never passes through a tool call, so none of it is below.")
    print("   `ctx_start` in the default report is where that floor shows up.")
    merged = {}
    for s in sessions:
        for label, size in s["attribution"].items():
            merged[label] = merged.get(label, 0) + size
    total = sum(merged.values()) or 1
    print(f"\n{'bucket':<16}{'bytes':>12}{'approx tok':>13}{'share':>8}")
    for label, size in sorted(merged.items(), key=lambda kv: -kv[1]):
        print(f"{label:<16}{size:>12,}{size / 3.6:>13,.0f}{size / total:>7.0%}")
    print(f"{'TOTAL':<16}{total:>12,}{total / 3.6:>13,.0f}{1:>7.0%}")

    orientation = merged.get("orientation", 0) / total
    adr = merged.get("adr", 0) / total
    discovery = merged.get("discovery", 0) / total
    print("\n   What each share argues for, in docs/agent/loop-goal.toml's [context] block:")
    print(f"   orientation {orientation:>4.0%}  -- if this is large, the pack itself is too wide:")
    print("                     narrow `modules`, `playbook` and `shapes`, and check")
    print("                     `python tools/orient.py --audit` for which section carries it.")
    print(f"   adr         {adr:>4.0%}  -- if this is large, whole ADRs are being read where")
    print("                     `adrs = [\"NNNN §N\"]` would have sliced one section.")
    print(f"   discovery   {discovery:>4.0%}  -- if this is large, the handoff's checklist items are")
    print("                     missing their file:line anchors, so every session re-derives them.")

    delegated = [(s["log"], a) for s in sessions for a in s["subagents"]]
    # The two buckets a subagent is allowed to take: finding where something is, and reading a
    # file the session will not edit. Everything else -- the item's own anchors, the code being
    # changed, the verification -- must be read by the session that writes the handoff.
    searching = merged.get("discovery", 0) + merged.get("source", 0)
    spawned = len({log for log, _ in delegated})
    print("\n== DELEGATED  (subagent transcripts captured beside the session's own)")
    if delegated:
        print(f"\n{'session':<32}{'agent':<26}{'calls':>7}{'peak ctx':>11}")
        for log, a in delegated:
            print(f"{log:<32}{a['agent']:<26}{a['calls']:>7}{a['peak_ctx']:>11,}")
        print(
            "\n   A subagent pays the same startup floor a session does, so a narrow lookup is\n"
            "   cheap only in the PARENT's context, never in absolute tokens. Peak context here\n"
            "   is what that floor actually cost."
        )
    else:
        print(
            "\n   Nothing. Either no session spawned a subagent, or these logs predate\n"
            "   tools/loop.py capturing them -- the two look identical from here, which is why\n"
            "   the capture exists."
        )
    print(
        f"\n   {spawned} of {len(sessions)} session(s) delegated anything, against "
        f"{searching:,} B ({searching / 3.6:,.0f} tok, {searching / total:.0%} of what was\n"
        f"   fetched) charged to their own windows by discovery and source reads. That is the\n"
        "   ceiling on what delegation could ever have moved, not a target: a subagent is for a\n"
        "   search over files this session will NOT open, and its startup floor makes a narrow\n"
        "   one a loss. The half of it that is the item's own anchors was never delegable.\n"
        "   docs/agent/session-prompt.md is where the rule and the safety boundary live."
    )


CALIBRATION = ROOT / "tools" / "data" / "calibration.json"


def calibrate(sessions, write):
    """Bytes per token for the orientation pack, regressed rather than assumed.

    `loop.py` records each session's pack size beside its transcript, and `ctx_start` is that
    session's measured opening context. Across sessions the pack is the only part of the floor
    that moves -- the harness prompt, the tool schemas, CLAUDE.md and AGENTS.md are the same
    bytes every time -- so the slope of ctx_start against pack bytes IS the ratio, and the
    intercept is the fixed floor underneath it. Two sessions with different packs is enough;
    more is better.

    A single guessed constant is what this replaces. `orient.py --audit` divided by 1.75 on the
    strength of one before-and-after, which put its own pack at 50k tokens when the transcripts
    said the whole session floor -- pack, prompt, schemas and all -- was 57k.
    """
    points = [(s["pack_bytes"], s["ctx_start"]) for s in sessions
              if s.get("pack_bytes") and s.get("ctx_start")]
    spread = {p for p, _ in points}
    if len(spread) < 2:
        print("== CALIBRATION")
        print(f"   {len(points)} session(s) recorded a pack size, {len(spread)} distinct.")
        print("   Two DIFFERENT pack sizes are the minimum for a slope. `loop.py` began")
        print("   recording them with the `loop_pack` line; a run over these logs will have")
        print("   them. Until then orient.py uses its default and says so.")
        return None

    n = len(points)
    mx = sum(p for p, _ in points) / n
    my = sum(c for _, c in points) / n
    sxx = sum((p - mx) ** 2 for p, _ in points)
    sxy = sum((p - mx) * (c - my) for p, c in points)
    if sxx == 0:
        return None
    slope = sxy / sxx                      # tokens per byte
    intercept = my - slope * mx            # the floor with no pack at all
    ratio = 1 / slope if slope > 0 else 0

    resid = sum((c - (intercept + slope * p)) ** 2 for p, c in points)
    tot = sum((c - my) ** 2 for _, c in points)
    r2 = 1 - resid / tot if tot else 0

    print("== CALIBRATION  (pack bytes -> opening context, regressed over "
          f"{n} session(s))")
    print(f"   bytes per token       {ratio:,.2f}")
    print(f"   fixed floor           {intercept:,.0f} tokens  "
          "(harness prompt + tool schemas + CLAUDE.md + AGENTS.md)")
    print(f"   fit                   R^2 {r2:.3f} over pack sizes "
          f"{min(spread):,} - {max(spread):,} B")
    print()
    print("   Every token of the pack is re-billed on every turn of the session, so at the")
    print(f"   measured {sum(s['calls'] for s in sessions) / len(sessions):.0f} calls a session, "
          "1,000 bytes of pack is about")
    print(f"   {1000 / ratio * sum(s['calls'] for s in sessions) / len(sessions):,.0f} "
          "billed tokens. That is the number to weigh a `[context]` selector against.")

    if write:
        CALIBRATION.parent.mkdir(parents=True, exist_ok=True)
        CALIBRATION.write_text(
            # `calls_per_session` too: `orient.py`'s audit already reads that key and only falls
            # back to a constant of its own when it is absent, and that constant said 98 when
            # these logs measured 71 -- which overstates what trimming the pack buys by a third.
            json.dumps({"bytes_per_token": round(ratio, 3),
                        "floor_tokens": round(intercept),
                        "r_squared": round(r2, 4),
                        "calls_per_session": round(sum(s["calls"] for s in sessions) / len(sessions)),
                        "sessions": n}, indent=2) + "\n",
            encoding="utf-8", newline="\n")
        print(f"\n   written to {CALIBRATION.relative_to(ROOT).as_posix()} -- "
              "orient.py --audit reads it from there.")
    else:
        print("\n   --write records this in tools/data/calibration.json, which is where")
        print("   orient.py --audit looks before falling back to its default.")
    return ratio


#: A session's fixed cost is meant to be FIXED -- that is the whole claim `loop.py`'s docstring
#: makes for starting a fresh session each time ("the per-session context cost is constant no
#: matter how many sessions run"). It stops being true the moment something a session writes is
#: also something every later session reads: the plan's status fields and the playbook both are,
#: and both are written by `session.py --wrap`. Growth there is not a big pack, it is a pack that
#: gets bigger with the number of sessions served -- the same shape AGENTS.md's priority ordering
#: calls a leak rather than a trade-off.
#:
#: It went unseen for a whole run once. `Open now` grew 4,106 -> 44,160 B over 21 sessions, about
#: 2 KB a session, until it was 48% of the orientation pack; every number in this script was
#: already being printed and none of them was that one. So this is a slope, not a size.
DRIFT_BYTES_PER_SESSION = 400

#: A near-zero slope is not the same as a pack that did not grow, and reporting only the slope
#: printed "flat enough -- the per-session cost is not growing" directly underneath
#: `69,962 -> 80,079 B`, a 14% rise over one run. A least-squares line through the points cannot
#: see a step: a section added once lifts every later point equally and bends the line barely at
#: all. Both shapes are worth naming and they are not the same defect -- a slope means something
#: a session WRITES is read by every session after it and grows without bound, while a step means
#: somebody added a section and it will sit there at exactly that size. 5 KB is roughly 1,250
#: tokens, re-billed on every call of every session, which is about one tool call's worth of
#: context given away for free on each of the ~66 a session makes.
STEP_BYTES = 5_000


def report_head(sessions):
    """What the calls before the first edit were reading, bucketed.

    `head` is the one part of a session's fixed cost that is supposed to already be paid. The
    driver orients the session and hands it the pack with the prompt, so the floor here is not
    "a few calls", it is zero. Whatever the gap turns out to be, this prints what it was spent
    on rather than how big it was: `orientation` in this table is a session re-reading a source
    the pack was built from, which is a different defect from a session reading source code it
    genuinely had to look up, and the two are fixed in different files.

    Sessions that never edited anything are left out -- `head` falls back to the whole session
    for those, which would price an aborted session as the most expensive orientation in the
    run."""
    heads = [s for s in sessions if s.get("head_buckets") and s["head"] < s["calls"]]
    if not heads:
        return
    totals = {}
    for s in heads:
        for label, n in s["head_buckets"].items():
            totals[label] = totals.get(label, 0) + n
    calls = sum(totals.values())
    if not calls:
        return
    n_sessions = len(heads)
    print(
        f"\n== WHERE THE HEAD CALLS GO  ({calls / n_sessions:.1f} a session before the first edit, "
        f"{n_sessions} session(s))"
    )
    for label, n in sorted(totals.items(), key=lambda kv: (-kv[1], kv[0])):
        print(f"   {label:<16}{n / n_sessions:>5.1f} a session{n / calls * 100:>5.0f}%")
    reread = totals.get("orientation", 0)
    if reread:
        with_reread = sum(1 for s in heads if s["head_buckets"].get("orientation"))
        print(
            f"\n   {reread:.0f} of those, across {with_reread} of {n_sessions} session(s), RE-READ AN "
            f"ORIENTATION SOURCE\n"
            f"   -- the pack, the handoff, the playbook, conventions.md, AGENTS.md -- which the\n"
            f"   driver had already piped in ahead of the prompt. Either the `[context]` manifest\n"
            f"   is missing a field the goal needs, or the pack carried it and it was not found.\n"
            f"   Those are opposite fixes, so read the calls in `.loop/logs/` before making one.\n"
            f"   A third case reads like either and is neither: the pack prints the REST of the\n"
            f"   handoff's group one 150-char line each, so a session taking a second slice pays\n"
            f"   one `handoff.md:\"## Next group\"` peek by design. That call is not a defect."
        )


def report_drift(sessions):
    """Is the pack growing with the number of sessions? A slope, printed only when there is one.

    The slope is fitted **inside each goal** and pooled: every session's pack is measured against
    its own goal's mean, so a chain switch -- which installs a new `[context]` manifest and can
    double the pack without any session writing a byte of it -- is a step between goals rather
    than a slope through them. Fitted across a switch, a goal boundary reads as a leak. Sessions
    whose pack event names no goal share one unnamed goal, and a window of only those is fitted
    whole."""
    pts = [(i, s["pack_bytes"], s.get("pack_goal") or "") for i, s in enumerate(sessions)
           if s.get("pack_bytes")]
    if len(pts) < 5:
        return
    n = len(pts)
    runs = []  # consecutive sessions on one goal: [(goal, [(x, y), ...]), ...]
    for x, y, goal in pts:
        if runs and runs[-1][0] == goal:
            runs[-1][1].append((x, y))
        else:
            runs.append((goal, [(x, y)]))
    num = denom = 0.0
    for _, run in runs:
        mx = sum(x for x, _ in run) / len(run)
        my = sum(y for _, y in run) / len(run)
        num += sum((x - mx) * (y - my) for x, y in run)
        denom += sum((x - mx) ** 2 for x, _ in run)
    if not denom:
        return
    slope = num / denom
    first, last = pts[0][1], pts[-1][1]
    # What the chain switches moved by themselves: each goal's first pack against the last one the
    # goal before it left.
    switched = sum(run[0][1] - prev[-1][1] for (_, prev), (_, run) in zip(runs, runs[1:]))
    print(f"\n== FIXED COST  (the orientation pack, first session to last)")
    print(f"   {first:,} -> {last:,} B, {slope:+,.0f} B a session inside a goal")
    if len(runs) > 1:
        names = " -> ".join(f"`{g}`" if g else "(unnamed)" for g, _ in runs)
        print(f"   {switched:+,} B of that is at {len(runs) - 1} chain switch(es), {names}:\n"
              f"   a new manifest, authored rather than accumulated, and narrowed only by whoever\n"
              f"   writes the goal.")
    if slope < DRIFT_BYTES_PER_SESSION:
        step = last - first - switched
        if step > STEP_BYTES:
            print(
                f"   NOT A LEAK, BUT NOT FLAT EITHER: the pack STEPPED {step:+,} B "
                f"({step / first * 100:+.0f}%) across this\n"
                f"   run while regressing at {slope:+,.0f} B a session, so a section was added "
                f"rather than\n"
                f"   accumulated. `python tools/orient.py --audit` says which one. Unlike a slope "
                f"this will\n"
                f"   not grow on its own, so it is a cut to make deliberately or to keep "
                f"deliberately."
            )
        else:
            print(
                "   flat enough -- the per-session cost is not growing with the number of sessions."
            )
        # The slope runs through the transcripts, and a transcript records the pack a session
        # already carried. A pass that adds a section therefore lands on disk a whole run before
        # it can bend this line, and "flat enough" read on its own is advice about a pack that no
        # longer exists. The projection further down already opens at the disk figure; say it here
        # too, where the reader is deciding whether there is anything to cut.
        live = live_ctx_start(sessions)
        if live and live["pack"] - last > DRIFT_BYTES_PER_SESSION:
            print(
                f"   BUT THE PACK ON DISK IS {live['pack']:,} B, {live['pack'] - last:+,} B past "
                f"the last one a\n"
                f"   session actually carried. That jump is not in the slope yet, so read this "
                f"again\n"
                f"   after the next run before concluding the pack is flat."
            )
        return
    print(
        f"   THIS IS A LEAK, NOT A BIG PACK. At {slope:,.0f} B a session the next {n} sessions pay\n"
        f"   {first + slope * 2 * n:,.0f} B each, and every byte is re-billed on every one of a\n"
        f"   session's calls. Something a session WRITES is being read by every session after it.\n"
        f"   `python tools/orient.py --audit` says which section, and it is usually the plan's\n"
        f"   status block or the playbook: `python tools/plan.py --check` and\n"
        f"   `python tools/playbook.py --check` price those two. A status field carrying a record\n"
        f"   of what landed belongs in `git log`; a finding belongs in a playbook bullet no goal\n"
        f"   ships until its file set implies it."
    )


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--run", help="limit to one run stamp, e.g. 20260825-015956")
    ap.add_argument(
        "--attribute",
        action="store_true",
        help="where the context went, by what fetched it, plus any subagent transcripts",
    )
    ap.add_argument(
        "--context-ceiling",
        type=int,
        help="context a session must stay under. Default: 80%% of the window the transcripts "
        "say the model had, leaving the last turn room to write the handoff.",
    )
    ap.add_argument("--json", action="store_true", help="print one JSON object instead")
    ap.add_argument(
        "--calibrate",
        action="store_true",
        help="regress pack bytes against opening context for a measured bytes-per-token",
    )
    ap.add_argument(
        "--write",
        action="store_true",
        help="with --calibrate, record the result in tools/data/calibration.json",
    )
    opts = ap.parse_args()

    if not LOGDIR.is_dir():
        sys.exit(f"no {LOGDIR.relative_to(ROOT)} -- nothing has been measured yet")
    pattern = f"{opts.run}-*.log" if opts.run else "*.log"
    # `<run>-console.log` is the run's own record of itself -- stamped plain text, not NDJSON, and
    # not one session. It lives in the same directory so a run prunes as a unit; it is not a
    # transcript, so it is not measured as one. `-run.log` and `-supervisor.log` are the same kind
    # of file under names an older driver gave it, and are still on disk in older runs.
    paths = sorted(p for p in LOGDIR.glob(pattern)
                   if not p.name.endswith(("-console.log", "-run.log", "-supervisor.log")))
    if not paths:
        sys.exit(f"no transcripts match {pattern} in {LOGDIR.relative_to(ROOT)}")

    sessions = [s for s in (read_session(p) for p in paths) if s]
    if not sessions:
        sys.exit(f"{len(paths)} transcript(s) found, none holding a completed assistant turn")
    t = totals(sessions)

    if opts.json:
        print(json.dumps({"sessions": sessions, "constants": t}, indent=2))
        return

    if opts.calibrate:
        calibrate(sessions, opts.write)
        return

    if opts.attribute:
        render_attribution(sessions)
        return

    # `N transcript(s)` reads as all-time and is not: the driver prunes `.loop/logs` to its newest
    # `--keep-runs` runs at every run start, so the set moves under a reader comparing this count --
    # or any "k of N session(s)" derived from it -- against an earlier report's. An optimization
    # pass recorded nearly filing a five-point regression that was only the window sliding.
    # Naming the runs makes two reports comparable, or visibly not; it is also the only place the
    # retained window is stated as a fact rather than as whatever `--keep-runs` was set to.
    runs = sorted({s["log"].rsplit("-", 1)[0] for s in sessions})
    window = runs[0] if len(runs) == 1 else f"{runs[0]} .. {runs[-1]}"
    print(f"== PER SESSION  ({len(sessions)} transcript(s) in .loop/logs, "
          f"{len(runs)} run(s): {window} -- older runs are pruned)")
    print(
        f"{'log':<28}{'calls':>6}{'/msg':>6}{'cmd/c':>7}{'head':>6}{'work':>6}{'tail':>6}"
        f"{'vfy':>5}{'cmt':>5}{'ctx end':>10}{'min':>7}{'$':>8}"
    )
    for s in sessions:
        mins = f"{s['duration_ms']/60000:.1f}" if s["duration_ms"] else "-"
        cost = f"{s['cost_usd']:.2f}" if s["cost_usd"] else "-"
        flag = "  COMPACTED" if s["compactions"] else ""
        print(
            f"{s['log']:<28}{s['calls']:>6}{s['per_message']:>6}{s['per_shell_call']:>7.2f}"
            f"{s['head']:>6}"
            f"{s['work']:>6}{s['tail']:>6}{s['verify_runs']:>5}{s['commits']:>5}"
            f"{s['ctx_end']:>10,}{mins:>7}{cost:>8}{flag}"
        )
        if not s["complete"]:
            print(f"{'':<28}(incomplete: no result event -- excluded from the constants)")

    if not t:
        print("\nno completed session to derive constants from; run the loop once and re-run this")
        return

    fixed = t["head"] + t["tail"]
    total_calls = fixed + t["work"]
    print(f"\n== CONSTANTS  (mean over {t['sessions']} complete session(s))")
    print(f"   seconds per tool call     {t['seconds_per_call']:.1f}")
    print(f"   share of clock on the API {t['api_share']*100:.0f}%")
    print(f"   context per tool call     {t['ctx_per_call']:,.0f} tokens")
    print(
        f"   fixed cost per session    {fixed:.0f} of {total_calls:.0f} calls "
        f"({fixed/total_calls*100:.0f}%) -- head {t['head']:.0f} + tail {t['tail']:.0f}"
    )
    print(f"   cost per session          ${t['cost_per_session']:.2f}")

    report_head(sessions)
    report_drift(sessions)

    # Two kinds of batching, and only one of them has ever happened. Across messages: 0 of 3,647
    # calls over one run, then 0 again over the next 19 sessions -- it does not happen and the
    # tooling stopped asking. Inside one shell call, a `;`/`&&` chain: 42% of shell calls, 3.3
    # commands each. Reporting only the first said "nothing was ever batched" at a session that
    # had just chained 39 commands into one call, and sent every goal author after a saving that
    # was already taken.
    chained = [s for s in sessions if s["per_shell_call"] > 1.05]
    if chained:
        mean = sum(s["per_shell_call"] for s in chained) / len(chained)
        print(
            f"\n   BATCHING IS IN THE SHELL, NOT ACROSS MESSAGES. {len(chained)} of "
            f"{len(sessions)} session(s) chained\n"
            f"   commands with `;`/`&&`, {mean:.2f} per shell call, while every message still "
            f"carried one\n"
            f"   tool call. That is the habit `peek.py` and AGENTS.md rule 2 ask for, so the "
            f"saving is\n"
            f"   taken -- do not read the 1.00 above as one going begging."
        )
    else:
        print(
            "\n   NOTHING WAS BATCHED, in either sense: no message carried two tool calls and no\n"
            "   shell call carried two commands. That is the one case where the clock above is\n"
            "   the worst case and the cheapest saving really is untaken."
        )

    # AGENTS.md rule 1, measured. It is here rather than in a linter because the rule is about
    # a risk that mostly does not fire -- a heredoc carrying Rust works until the day an
    # apostrophe in a doc comment ends the quote early -- so what a run needs is the count, not
    # a gate that would fail a session for a `.agent-tmp/` scratch file.
    writers = [s for s in sessions if s["shell_writes"]]
    if writers:
        spliced = sum(1 for s in sessions if "splice" in json.dumps(s.get("attribution", {})))
        print(
            f"\n   THE SHELL IS CARRYING FILE CONTENT. {sum(s['shell_writes'] for s in writers)} "
            f"call(s) across {len(writers)} of\n"
            f"   {len(sessions)} session(s) used a heredoc, a `>` redirect or a `sed -i` where "
            f"AGENTS.md rule 1\n"
            f"   asks for Write/Edit or `bun nv splice --patch`. The shell parses "
            f"apostrophes\n"
            f"   and backticks before it runs anything, so this is the spelling that fails on a "
            f"doc\n"
            f"   comment rather than on anything the session did wrong."
            + (f" ({spliced} session(s) used a splice.)" if spliced else "")
        )

    window = context_window(sessions)
    ceiling = opts.context_ceiling or CONTEXT_CEILING
    source = (
        "given on the command line"
        if opts.context_ceiling
        else "the fixed quality ceiling -- an agent degrades well before its window is full"
    )
    if window and window < ceiling:
        ceiling, source = window, f"the model's {window:,}-token window, smaller than the quality ceiling"

    over = [s for s in sessions if s["ctx_end"] > ceiling]
    compacted = [s for s in sessions if s["compactions"]]
    print(f"\n== PROJECTION  (ceiling {ceiling:,}, {source})")
    if over:
        print(
            f"   {len(over)} of {len(sessions)} session(s) ALREADY FINISHED OVER THE CEILING, doing one\n"
            f"   slice each -- largest {max(s['ctx_end'] for s in sessions):,}. Until that comes down,\n"
            f"   the lever is reading less per session, not doing more per session."
        )
    else:
        print(f"   largest context any session here reached: {max(s['ctx_end'] for s in sessions):,}")
    # The projection is advice about the NEXT run, so it opens where the next session will open,
    # not where the last ones did. The pack on disk is the only part of that floor anyone moves.
    live = live_ctx_start(sessions)
    if live and abs(live["tokens"] - t["ctx_start"]) > 2_000:
        print(
            f"   the pack on disk is now {live['pack']:,} B, so the NEXT session opens at "
            f"{live['tokens']:,.0f},\n   not the {t['ctx_start']:,.0f} these transcripts averaged. "
            "Everything below uses the former."
        )
        t = dict(t, ctx_start=live["tokens"])
    budget = ceiling - t["ctx_start"]
    print(
        f"   a session starts at {t['ctx_start']:,.0f} (prompt + AGENTS.md + orientation), leaving "
        f"{budget:,.0f}\n   for growth -- about {budget/t['ctx_per_call']:.0f} calls at the measured "
        f"{t['ctx_per_call']:,.0f} tokens a call"
    )
    if compacted:
        print(f"   {len(compacted)} session(s) COMPACTED -- the ceiling is already too high")
    print(f"   {'slices':>7}{'calls':>7}{'minutes':>9}{'vs solo':>9}{'tokens':>9}{'ctx end':>10}")
    rows = project(t, ceiling)
    for r in rows:
        note = "  over ceiling" if r["over_ceiling"] else ""
        print(
            f"   {r['slices']:>7}{r['calls']:>7}{r['minutes']:>9.0f}"
            f"{r['speedup']:>8.2f}x{r['token_ratio']:>8.2f}x{r['ctx_end']:>10,}{note}"
        )

    # Three readings of the same curve, because they optimise three different things. None of
    # them is a cap to install: AGENTS.md step 2 replaced the slice count with the 120k gate,
    # and takes only the ceiling from here. loop-authoring.md § 1 names the lever each shape wants.
    safe = [r for r in rows if not r["over_ceiling"]]
    cheapest = [r for r in safe if r["token_ratio"] <= 1.0]
    knee = [
        r
        for r, prev in zip(safe[1:], safe)
        if r["speedup"] - prev["speedup"] >= 0.10
    ]
    print("\n== GROUP CURVE  what the curve says, not a cap to install")
    if not safe:
        print(
            "   NONE. Even a single slice projects past the ceiling, so there is no group size to\n"
            "   pick: there is nothing to group, and the work is cutting what a session reads --\n"
            "   not counting slices. The projection above charges every extra slice a full fresh\n"
            "   read, which is the right assumption for an unrelated slice and pessimistic for one\n"
            "   sharing a file set -- so re-run this after the first session that lands well under\n"
            "   the ceiling."
        )
    if cheapest:
        r = cheapest[-1]
        print(
            f"   cheapest      {r['slices']} slices -- {r['speedup']:.2f}x faster, "
            f"{r['token_ratio']:.2f}x tokens. Past here a group costs more than separate sessions."
        )
    if knee:
        r = knee[-1]
        print(
            f"   best value    {r['slices']} slices -- {r['speedup']:.2f}x faster, "
            f"{r['token_ratio']:.2f}x tokens. The last slice worth +0.10x; after it the curve flattens."
        )
    if safe:
        r = safe[-1]
        print(
            f"   fastest safe  {r['slices']} slices -- {r['speedup']:.2f}x faster, "
            f"{r['token_ratio']:.2f}x tokens, ending at {r['ctx_end']:,} against the ceiling."
        )
    if safe:
        print(
            "\n   These are readings of the curve, not caps to install. AGENTS.md § Session\n"
            "   workflow step 2 holds no slice count any more -- the 120k gate replaced it, because\n"
            "   a slice's cost is not fixed and a count prices every slice as the most expensive\n"
            "   one. What step 2 does take from here is the CEILING, and only after a run that\n"
            "   changed what a session reads. docs/agent/loop-authoring.md § Measure first says\n"
            "   which lever each shape wants."
        )


if __name__ == "__main__":
    main()
