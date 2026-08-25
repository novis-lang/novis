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
  calls/msg       tool calls per assistant message; 1.00 means nothing was ever batched
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
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOGDIR = ROOT / ".loop" / "logs"

# A tail begins at the first verification call: everything from there on is verify, docs,
# handoff and commit, which a group pays once no matter how many slices sit in front of it.
VERIFY_MARKERS = ("verify.py", "tools/verify")
MUTATORS = ("Edit", "Write", "NotebookEdit")

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
# it. Buckets are matched in order, first hit wins.

BUCKETS = (
    ("orientation", ("orient.py", "brief.py", "docs/agent/", "loop-goal", "handoff", "playbook",
                     "conventions", "AGENTS.md", "CLAUDE.md")),
    ("adr", ("docs/adr/",)),
    ("plan + spec", ("implementation-plan", "docs/plan/", "docs/spec/", "plan.py")),
    ("build + test", ("verify.py", "cargo ", "mwl test", "mwl run", "loop.py")),
    ("git", ("git ",)),
    ("discovery", ("grep", "rg ", "find ", " ls ", "glob", "Glob", "Grep")),
    ("source", ("crates/", "benches/", "tests/", "examples/", "tools/", "fuzz/")),
)


def bucket_of(name, text):
    if name in MUTATORS:
        return "writing"
    for label, needles in BUCKETS:
        if any(nd in text for nd in needles):
            return label
    return "other"


def subagent_cost(path):
    """The sessions this transcript delegated to, out of `.loop/logs/<stem>.subagents/`.

    A subagent's turns never appear in the parent's stream, so without this a delegated read
    is a session that mysteriously did a great deal with very few calls. `tools/loop.py`
    copies the harness's own transcripts there; if the directory is absent, nothing was
    delegated -- or the run predates that capture, which is why the count is reported
    separately rather than folded into the session's own figures."""
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
    names, attribution = {}, {}
    pack_bytes = 0
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue  # a truncated final line is normal for a killed run
        kind = event.get("type")
        if kind == "loop_pack":
            pack_bytes = event.get("bytes") or 0
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
                label = names.get(str(block.get("tool_use_id")), "other")
                body = block.get("content")
                text = body if isinstance(body, str) else json.dumps(body)
                attribution[label] = attribution.get(label, 0) + len(text)
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
                label = bucket_of(c.get("name"), call_text((c.get("name"), c.get("input"))))
                names[str(c.get("id"))] = label
                # An Edit's *input* is the new code, which is real context the session spent.
                if c.get("name") in MUTATORS:
                    attribution[label] = attribution.get(label, 0) + len(
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
    mutations = [i for i, (name, _) in enumerate(calls) if name in MUTATORS]
    verifies = [i for i, t in enumerate(texts) if any(m in t for m in VERIFY_MARKERS)]
    commits = [i for i, t in enumerate(texts) if "git commit" in t]

    n = len(calls)
    head = mutations[0] if mutations else n
    tail_start = verifies[0] if verifies else n
    # A session that verified before it edited anything did something unusual; fold the
    # oddity into `work` rather than reporting a negative phase.
    tail_start = max(tail_start, head)

    drops = [
        (i, contexts[i - 1], contexts[i])
        for i in range(1, len(contexts))
        if contexts[i] < contexts[i - 1] * 0.6
    ]

    return {
        "log": path.name,
        "calls": n,
        "per_message": round(sum(per_message) / len(per_message), 2) if per_message else 0.0,
        "head": head,
        "work": tail_start - head,
        "tail": n - tail_start,
        "verify_runs": len(verifies),
        "commits": len(commits),
        "ctx_start": contexts[0] if contexts else 0,
        "ctx_end": max(contexts) if contexts else 0,
        "pack_bytes": pack_bytes,
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
    if delegated:
        print("\n== DELEGATED  (subagent transcripts captured beside the session's own)")
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
            "\n== DELEGATED\n\n   Nothing. Either no session spawned a subagent, or these logs predate\n"
            "   tools/loop.py capturing them -- the two look identical from here, which is why\n"
            "   the capture exists."
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
            json.dumps({"bytes_per_token": round(ratio, 3),
                        "floor_tokens": round(intercept),
                        "r_squared": round(r2, 4),
                        "sessions": n}, indent=2) + "\n",
            encoding="utf-8", newline="\n")
        print(f"\n   written to {CALIBRATION.relative_to(ROOT).as_posix()} -- "
              "orient.py --audit reads it from there.")
    else:
        print("\n   --write records this in tools/data/calibration.json, which is where")
        print("   orient.py --audit looks before falling back to its default.")
    return ratio


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
    paths = sorted(LOGDIR.glob(pattern))
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

    print(f"== PER SESSION  ({len(sessions)} transcript(s) in .loop/logs)")
    print(
        f"{'log':<28}{'calls':>6}{'/msg':>6}{'head':>6}{'work':>6}{'tail':>6}"
        f"{'vfy':>5}{'cmt':>5}{'ctx end':>10}{'min':>7}{'$':>8}"
    )
    for s in sessions:
        mins = f"{s['duration_ms']/60000:.1f}" if s["duration_ms"] else "-"
        cost = f"{s['cost_usd']:.2f}" if s["cost_usd"] else "-"
        flag = "  COMPACTED" if s["compactions"] else ""
        print(
            f"{s['log']:<28}{s['calls']:>6}{s['per_message']:>6}{s['head']:>6}"
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

    batched = [s for s in sessions if s["per_message"] > 1.05]
    if not batched:
        print(
            "\n   NOTHING WAS EVER BATCHED. Every message carried exactly one tool call, so the\n"
            "   clock above is the worst case and the cheapest available saving is untaken.\n"
            "   session-prompt.md's clock section already asks for this."
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

    # Three defensible caps, because they optimise three different things. Which one is
    # right is the author's call, not this script's: loop-authoring.md names the tradeoff.
    safe = [r for r in rows if not r["over_ceiling"]]
    cheapest = [r for r in safe if r["token_ratio"] <= 1.0]
    knee = [
        r
        for r, prev in zip(safe[1:], safe)
        if r["speedup"] - prev["speedup"] >= 0.10
    ]
    print("\n== CAPS  three answers, optimising three different things")
    if not safe:
        print(
            "   NONE. Even a single slice projects past the ceiling, so there is no group size to\n"
            "   pick: the cap is one slice, and the work is cutting what a session reads. The\n"
            "   projection above charges every extra slice a full fresh read, which is the right\n"
            "   assumption for an unrelated slice and pessimistic for one sharing a file set --\n"
            "   so re-run this after the first session that lands well under the ceiling."
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
            "\n   Pick one, put it in AGENTS.md § Session workflow step 2, and say in the commit which\n"
            "   of the three it is and why. If the cap there matches none of them, it predates this\n"
            "   measurement: docs/agent/loop-authoring.md § Measure first says what to do about that."
        )


if __name__ == "__main__":
    main()
