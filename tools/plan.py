#!/usr/bin/env python3
"""Read and rewrite one field of the implementation plan's leading status block.

That block is the one doc every session edits, and editing it by hand costs more turns than
anything else a session does: locate the field's exact bytes with a few narrow reads, write an
old block and a new block, splice, then read it back to confirm. Measured across the loop's
sessions, that cycle averaged **11.6 tool calls per session**. Named field in, new text in,
done -- this is that cycle as one call.

    python tools/plan.py                             # every field, with its size
    python tools/plan.py --get "Open now"            # one field's text, unwrapped
    python tools/plan.py --set "Open now" --from <file>    # replace one field's text

`--set` takes the replacement from a *file* rather than the command line, for the reason
docs/agent/commands.md gives: a shell parses its argument before anything runs, and this
repository's prose is full of apostrophes and backticks. Write the new text with the Write
tool, pass the path.

It rewrites exactly one field and refuses everything else. A field name that is not already in
the block is an error, not an insertion -- the field set is fixed on purpose (the comment above
the block says so), and a tool that could add one would be a tool that could grow it forever.

`--set` re-wraps the field it writes to a canonical width; every other byte of the file, the
blank `>` separators included, is left exactly as it was. So the diff of a `--set` is always
one field and never the whole block.
"""

from __future__ import annotations

import argparse
import re
import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "implementation-plan.md"

WIDTH = 100  # including the "> " prefix, matching what is already in the file
FIELD_RE = re.compile(r"^> \*\*([^*:]+):\*\*\s*(.*)$")


def load():
    text = PLAN.read_text(encoding="utf-8")
    # newline='' on write preserves the file's existing endings; splitlines here keeps us
    # from caring which they are.
    return text, text.split("\n")


def find_fields(lines):
    """(name, first line index, one-past-last line index, joined text) per field, in order.

    A field's span ends at its last line with text on it. A bare `>` between two fields is a
    separator that belongs to the block, not to the field above it -- folding it into the span
    would mean every `--set` silently deleted the blank line under the field it rewrote."""
    fields = []
    started = False
    for i, raw in enumerate(lines):
        if raw.startswith(">"):
            started = True
            m = FIELD_RE.match(raw)
            if m:
                fields.append([m.group(1).strip(), i, i + 1, [m.group(2).strip()]])
            elif fields:
                body = re.sub(r"^> ?", "", raw).strip()
                if body:
                    fields[-1][2] = i + 1
                    fields[-1][3].append(body)
        elif started:
            break
    return [(n, a, b, " ".join(p).strip()) for n, a, b, p in fields]


def render(name, text):
    """`> **Name:** text`, wrapped the way the block already is."""
    return textwrap.wrap(
        f"**{name}:** {text}",
        width=WIDTH - 2,
        break_long_words=False,
        break_on_hyphens=False,
    )


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--get", metavar="FIELD")
    ap.add_argument("--set", metavar="FIELD", dest="set_field")
    ap.add_argument("--from", metavar="FILE", dest="source")
    opts = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    if not PLAN.exists():
        print(f"plan.py: no {PLAN.relative_to(ROOT).as_posix()}")
        return 2

    text, lines = load()
    fields = find_fields(lines)
    if not fields:
        print("plan.py: no `> **Field:**` status block at the top of "
              f"{PLAN.relative_to(ROOT).as_posix()} -- read it directly")
        return 2
    names = [n for n, _, _, _ in fields]

    if opts.get:
        for name, _a, _b, body in fields:
            if name.lower() == opts.get.lower():
                print(body)
                return 0
        print(f"plan.py: no field {opts.get!r}. The block has: {', '.join(names)}")
        return 1

    if not opts.set_field:
        print(f"{PLAN.relative_to(ROOT).as_posix()} status block: {len(fields)} fields")
        for name, a, _b, body in fields:
            print(f"  {name:<20} {len(body.encode('utf-8')):>6} bytes   (line {a + 1})")
        print("\n--get <field> for one field's text; "
              "--set <field> --from <file> to replace it.")
        print("Changing several fields at the end of a session? `python tools/session.py --wrap`")
        print("takes them all in one file, with the handoff and the commits, in one call.")
        return 0

    if not opts.source:
        print("plan.py: --set needs --from <file> holding the new text")
        return 2
    src = Path(opts.source)
    if not src.exists():
        print(f"plan.py: no such file: {opts.source}")
        return 2

    target = None
    for name, a, b, body in fields:
        if name.lower() == opts.set_field.lower():
            target = (name, a, b, body)
            break
    if target is None:
        print(f"plan.py: no field {opts.set_field!r} in the status block, and this tool does "
              f"not add one -- the field set is fixed. It has: {', '.join(names)}")
        return 1

    name, start, end, old_body = target
    new_body = " ".join(src.read_text(encoding="utf-8").split())
    if not new_body:
        print(f"plan.py: {opts.source} is empty -- refusing to blank the {name!r} field")
        return 1

    rewritten = lines[:start] + ["> " + ln for ln in render(name, new_body)] + lines[end:]
    PLAN.write_text("\n".join(rewritten), encoding="utf-8", newline="")

    old_n = len(old_body.encode("utf-8"))
    new_n = len(new_body.encode("utf-8"))
    print(f"plan.py: {name} rewritten, {old_n} -> {new_n} bytes "
          f"({end - start} -> {len(render(name, new_body))} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
