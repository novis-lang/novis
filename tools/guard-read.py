#!/usr/bin/env python3
"""AGENTS.md rule 3, enforced on the harness's own `Read` tool.

Rule 3 is *read a big file in the region you need*, and `tools/peek.py` has enforced it on its own
side since it existed -- a bare `path` over `--max-lines` prints the file's size and refuses. The
harness's `Read` tool has no such floor, so it is the one way a whole 5,000-line file still reaches
a session's context. Measured over the 72 sessions in `.loop/logs`: only 16 `Read` calls in total
against 1,309 `peek.py` calls, but the largest single tool result of the whole run was one of them
-- 35,040 bytes of `crates/nvs-server/src/serve.rs` -- with a 26,147-byte `mount.rs` behind it, and
27 whole-file reads of the 177 KB `docs/agent/loop-goal.toml` across the same logs.

So this is a `PreToolUse` hook, wired in `.claude/settings.json`. It denies exactly one thing: a
`Read` of a file longer than `peek.DEFAULT_MAX_LINES` with no `offset`/`limit` narrowing it. A
denial costs a round trip, which is the currency this whole pass is trying to save, so the
threshold is deliberately the same 400 lines `peek.py` already uses -- one rule, one number, and
its home is `peek.py` -- and the reason names the two calls that answer the question instead.
At the measured rate it fires about five times in seventy sessions and saves 20-35 KB each time.

Harness-specific by necessity, not by design: `Read` is Claude Code's tool. Every other harness
gets the same floor from `peek.py`, which is why the number lives there and this file imports it.
`cat` and `sed -n '1,9999p'` through Bash are the hole this does not close; `peek.py`'s own footer
is what argues a session out of those.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import peek  # noqa: E402  -- same directory; rule 3's line count has one home and it is there

ROOT = Path(__file__).resolve().parent.parent


def deny(reason: str) -> None:
    json.dump(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        },
        sys.stdout,
    )
    sys.stdout.write("\n")


def main() -> int:
    """Anything unexpected allows the call. A hook that fails closed on a malformed payload would
    take a session down over a field name, and this one is a budget guard, not a safety gate."""
    try:
        event = json.load(sys.stdin)
    except (ValueError, OSError):
        return 0
    if event.get("tool_name") != "Read":
        return 0
    args = event.get("tool_input") or {}
    if args.get("offset") or args.get("limit"):
        return 0
    raw = str(args.get("file_path") or "")
    if not raw:
        return 0
    try:
        path = Path(raw)
        text = path.read_text(encoding="utf-8", errors="replace")
    except (OSError, ValueError):
        return 0
    lines = text.count("\n") + 1
    if lines <= peek.DEFAULT_MAX_LINES:
        return 0
    try:
        shown = path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        shown = raw
    deny(
        f"{shown} is {lines:,} lines ({len(text.encode('utf-8')):,} bytes) and this Read names no "
        f"region, so all of it would enter the context. AGENTS.md rule 3: read a big file in the "
        f"region you need.\n"
        f"  python tools/peek.py --outline {shown}          # the seams, with line numbers\n"
        f"  python tools/peek.py {shown}:120-160            # a region, or :@symbol, or :re:pattern\n"
        f"Both take as many targets as you have questions, in one call. `Read` with an explicit "
        f"`offset`/`limit` is not blocked either."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
