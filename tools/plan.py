#!/usr/bin/env python3
"""The one API over the implementation plan: its status block, and its milestones.

The plan is three kinds of document with three lifetimes, and since the split they are three
places on disk:

    docs/implementation-plan.md   the status block, rewritten every session, plus the milestone
                                  table that routes to the rest. This is the index; every link
                                  from an ADR still lands here.
    docs/plan/mN.md               one milestone each. Read one at a time, amended when an ADR
                                  reopens one.
    docs/plan/design.md           the decisions taken before M0, the architecture and the
                                  verification strategy. Near-static; almost never read.

Measured over six days the three grew x10, x2.7 and +8% respectively -- so they are separated by
how often they change, and this tool is what keeps a caller from having to know which is which.

    python tools/plan.py                            # fields with sizes, milestones with sizes
    python tools/plan.py --get "Open now"           # one field's text, unwrapped
    python tools/plan.py --set "Open now" --from F  # replace one field's text
    python tools/plan.py --show M8                  # one milestone, whole
    python tools/plan.py --show M8:verify           # only its acceptance paragraph
    python tools/plan.py --show M8:lead             # only its opening paragraph
    python tools/plan.py --amend M8 --from F        # replace one milestone's body
    python tools/plan.py --check                    # sizes against the aim, index against disk

`--set` and `--amend` take the replacement from a *file* rather than the command line, for the
reason docs/agent/commands.md gives: a shell parses its argument before anything runs, and this
repository's prose is full of apostrophes and backticks. Write the new text with the Write tool,
pass the path.

`--set` rewrites exactly one field and refuses everything else. A field name that is not already
in the block is an error, not an insertion -- the field set is fixed on purpose (the comment above
the block says so), and a tool that could add one would be a tool that could grow it forever. It
re-wraps the field it writes to a canonical width; every other byte of the file, the blank `>`
separators included, is left exactly as it was, so the diff of a `--set` is always one field.

`--amend` is the same contract for a milestone: it replaces the body under that milestone's H1 and
touches nothing else. It will not create a milestone. A **new** milestone is a decision, not a form
to fill in -- write the file and add its index row by hand, and `--check` will tell you if you got
one of the two wrong.

**Nothing here refuses over a length.** `--check` prices every field against the aim the plan's own
comment states and says what it costs a session; it exits 0 either way, for the reason
docs/agent/doc-style.md gives about length tripwires. A number is a report to weigh, not a gate.
"""

from __future__ import annotations

import argparse
import re
import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "implementation-plan.md"
PLAN_DIR = ROOT / "docs" / "plan"
DESIGN = PLAN_DIR / "design.md"

WIDTH = 100  # including the "> " prefix, matching what is already in the file
FIELD_RE = re.compile(r"^> \*\*([^*:]+):\*\*\s*(.*)$")

#: An index row: `| 1 | [M4S](plan/m4s.md) | The `Core` API contract … | ~1.5 |`
#: The leading **order** cell is what says what comes next, and a milestone's number is its identity
#: rather than its position (implementation-plan.md says so where the table is). It is optional here
#: so a table written before that column, or a row that never gets one, still parses -- and it is
#: matched rather than skipped so `--check` can report the schedule instead of only the roster.
#: The last cell is the loop-day projection (docs/plan/velocity.md owns what it means) and is
#: optional too. Both are matched separately rather than swept into the title, because `--check`
#: compares the title against the milestone file's H1 character for character and would otherwise
#: report every row as drifted.
ROW_RE = re.compile(
    r"^\|(?:\s*([^|\[]*?)\s*\|)?\s*\[(M\d+[A-Z]?)\]\((plan/[^)]+)\)"
    r"\s*\|\s*([^|]*?)\s*\|(?:\s*([^|]*?)\s*\|)?\s*$"
)

#: A milestone file's H1: `# M4S — The `Core` API contract and its pure half (~5 weeks)`
H1_RE = re.compile(r"^#\s+(M\d+[A-Z]?)\s*—\s*(.*)$")

#: Fallback if the plan's leading comment stops stating one. That comment is the aim's one home.
FIELD_AIM_FALLBACK = 400


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


def field_aim(text=None):
    """The per-field byte aim, read out of the plan's own leading comment."""
    if text is None:
        text = PLAN.read_text(encoding="utf-8")
    m = re.search(r"Aim for ~(\d+) bytes a field", text)
    return int(m.group(1)) if m else FIELD_AIM_FALLBACK


# ----------------------------------------------------------------------- milestones


def milestones(lines=None):
    """Every milestone the index routes to, in index order.

    The index table is the roster; a file under `docs/plan/` that no row names is reported by
    `--check` rather than silently included, because a milestone nothing links to is a milestone
    nobody reads. Title and state are *not* stored here twice -- the row carries the title, the
    file's H1 carries it too, and `--check` is what keeps the two from drifting."""
    if lines is None:
        _text, lines = load()
    found = []
    for i, raw in enumerate(lines):
        m = ROW_RE.match(raw)
        if m:
            found.append(
                {
                    "id": m.group(2),
                    "title": m.group(4),
                    "order": (m.group(1) or "").strip(),
                    "loop_days": (m.group(5) or "").strip(),
                    "path": ROOT / "docs" / m.group(3),
                    "rel": "docs/" + m.group(3),
                    "line": i + 1,
                }
            )
    return found


def resolve(mid, index=None):
    """`m8` / `M8` / `m4s` -> that milestone's entry, or None."""
    want = mid.strip().upper()
    for m in index if index is not None else milestones():
        if m["id"] == want:
            return m
    return None


def body_of(entry):
    """A milestone file's text below its H1."""
    text = entry["path"].read_text(encoding="utf-8")
    lines = text.split("\n")
    for i, raw in enumerate(lines):
        if H1_RE.match(raw):
            return "\n".join(lines[i + 1 :]).strip("\n")
    return text.strip("\n")


def lead_paragraph(entry):
    """The first paragraph of a milestone -- what it is, before the detail."""
    buf = []
    for raw in body_of(entry).split("\n"):
        if not raw.strip():
            if buf:
                break
            continue
        buf.append(raw.strip())
    return " ".join(buf)


def verify_paragraph(entry):
    """The `**Verify:**` / `**Verified:**` paragraph -- a milestone's acceptance test."""
    lines = body_of(entry).split("\n")
    for i, raw in enumerate(lines):
        if raw.startswith(("**Verify:**", "**Verified:**")):
            buf = []
            while i < len(lines) and lines[i].strip():
                buf.append(lines[i].strip())
                i += 1
            return " ".join(buf)
    return ""


def write_body(entry, body):
    """Replace everything under the H1, keeping the heading byte for byte."""
    lines = entry["path"].read_text(encoding="utf-8").split("\n")
    for i, raw in enumerate(lines):
        if H1_RE.match(raw):
            head = lines[: i + 1]
            break
    else:
        head = [f"# {entry['id']} — {entry['title']}"]
    entry["path"].write_text(
        "\n".join(head) + "\n\n" + body.strip("\n") + "\n", encoding="utf-8", newline="\n"
    )


# ---------------------------------------------------------------------------- report


def nbytes(text):
    return len(text.encode("utf-8"))


def report_index(fields, index, aim):
    print(f"{PLAN.relative_to(ROOT).as_posix()} status block: {len(fields)} fields "
          f"(aim ~{aim} bytes each)")
    total = 0
    for name, a, _b, body in fields:
        n = nbytes(body)
        total += n
        over = f"  {n / aim:.0f}x aim" if n > aim * 1.5 else ""
        print(f"  {name:<20} {n:>6} bytes   (line {a + 1}){over}")
    print(f"  {'':<20} {total:>6} bytes   read and rewritten every session")

    print(f"\ndocs/plan/: {len(index)} milestones, "
          f"{sum(nbytes(m['path'].read_text(encoding='utf-8')) for m in index if m['path'].exists()):>6}"
          " bytes in total")
    for m in index:
        size = nbytes(m["path"].read_text(encoding="utf-8")) if m["path"].exists() else 0
        flag = "" if m["path"].exists() else "   !! missing"
        print(f"  {m['id']:<6} {size:>6} bytes   {m['rel']}{flag}")
    if DESIGN.exists():
        print(f"  design {nbytes(DESIGN.read_text(encoding='utf-8')):>6} bytes   "
              f"{DESIGN.relative_to(ROOT).as_posix()}")

    print("\n--get <field> / --set <field> --from <file> for the status block;")
    print("--show M8 / --show M8:verify / --amend M8 --from <file> for a milestone;")
    print("--check prices the block and checks the index against what is on disk.")
    print("Changing several of these at the end of a session? `python tools/session.py --wrap`")
    print("takes them all in one file, with the handoff and the commits, in one call.")


def run_check(fields, index, aim):
    """Report only. Nothing here exits non-zero over a size."""
    problems = []

    total = sum(nbytes(b) for _n, _a, _b2, b in fields)
    print(f"status block: {total} bytes across {len(fields)} fields, aim ~{aim} each "
          f"(~{aim * len(fields)})")
    for name, _a, _b, body in fields:
        n = nbytes(body)
        if n > aim * 1.5:
            print(f"  {name:<20} {n:>6} bytes   {n / aim:.0f}x the aim")
    print("  Every one of these is shipped into every session by orient.py and brief.py.")
    print("  Nothing refuses an oversized field -- this is a number to weigh, not a gate.")

    print("\nindex vs disk:")
    seen = set()
    for m in index:
        seen.add(m["path"].resolve())
        if not m["path"].exists():
            problems.append(f"{m['id']}: index row points at {m['rel']}, which does not exist")
            continue
        first = m["path"].read_text(encoding="utf-8").split("\n")[0]
        h1 = H1_RE.match(first)
        if not h1:
            problems.append(f"{m['id']}: {m['rel']} does not open with `# {m['id']} — <title>`")
        else:
            if h1.group(1) != m["id"]:
                problems.append(f"{m['id']}: {m['rel']} calls itself {h1.group(1)}")
            if h1.group(2).strip() != m["title"]:
                problems.append(
                    f"{m['id']}: the index row and the file's H1 have drifted apart\n"
                    f"      index: {m['title']}\n      file:  {h1.group(2).strip()}"
                )
        if not verify_paragraph(m):
            problems.append(f"{m['id']}: no `**Verify:**` paragraph -- it has no acceptance test")

    for path in sorted(PLAN_DIR.glob("m*.md")):
        if path.resolve() not in seen:
            problems.append(f"{path.relative_to(ROOT).as_posix()}: on disk, but no index row "
                            "names it, so nothing links to it")

    if problems:
        for p in problems:
            print(f"  !! {p}")
    else:
        print(f"  {len(index)} rows, {len(index)} files, titles matching, every one with a "
              "`**Verify:**`")
    return 0


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--get", metavar="FIELD")
    ap.add_argument("--set", metavar="FIELD", dest="set_field")
    ap.add_argument("--show", metavar="M[:lead|:verify]")
    ap.add_argument("--amend", metavar="M")
    ap.add_argument("--check", action="store_true")
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
    index = milestones(lines)
    aim = field_aim(text)
    names = [n for n, _, _, _ in fields]

    if not fields:
        print("plan.py: no `> **Field:**` status block at the top of "
              f"{PLAN.relative_to(ROOT).as_posix()} -- read it directly")
        return 2

    # ------------------------------------------------------------------ milestones

    if opts.show:
        spec, _, part = opts.show.partition(":")
        entry = resolve(spec, index)
        if entry is None:
            print(f"plan.py: no milestone {spec!r}. The index has: "
                  f"{', '.join(m['id'] for m in index)}")
            return 1
        if not entry["path"].exists():
            print(f"plan.py: {entry['rel']} does not exist (the index row is at "
                  f"{PLAN.relative_to(ROOT).as_posix()}:{entry['line']})")
            return 1
        if part in ("", "all"):
            print(f"-- {entry['rel']}")
            print()
            print(entry["path"].read_text(encoding="utf-8").rstrip("\n"))
        elif part == "lead":
            print(lead_paragraph(entry))
        elif part == "verify":
            got = verify_paragraph(entry)
            if not got:
                print(f"plan.py: {entry['id']} has no `**Verify:**` paragraph")
                return 1
            print(got)
        else:
            print(f"plan.py: unknown part {part!r} -- use `:lead`, `:verify`, or no suffix")
            return 2
        return 0

    if opts.amend:
        entry = resolve(opts.amend, index)
        if entry is None:
            print(f"plan.py: no milestone {opts.amend!r} in the index, and this tool does not "
                  f"add one. It has: {', '.join(m['id'] for m in index)}")
            return 1
        if not opts.source:
            print("plan.py: --amend needs --from <file> holding the new body")
            return 2
        src = Path(opts.source)
        if not src.exists():
            print(f"plan.py: no such file: {opts.source}")
            return 2
        new = src.read_text(encoding="utf-8").strip("\n")
        if not new.strip():
            print(f"plan.py: {opts.source} is empty -- refusing to blank {entry['id']}")
            return 1
        old_n = nbytes(body_of(entry)) if entry["path"].exists() else 0
        write_body(entry, new)
        print(f"plan.py: {entry['id']} rewritten in {entry['rel']}, "
              f"{old_n} -> {nbytes(new)} bytes")
        if not verify_paragraph(entry):
            print("plan.py: !! the new body has no `**Verify:**` paragraph -- a milestone with "
                  "no acceptance test is one nothing can call done")
        return 0

    if opts.check:
        return run_check(fields, index, aim)

    # ------------------------------------------------------------------ status block

    if opts.get:
        for name, _a, _b, body in fields:
            if name.lower() == opts.get.lower():
                print(body)
                return 0
        print(f"plan.py: no field {opts.get!r}. The block has: {', '.join(names)}")
        return 1

    if not opts.set_field:
        report_index(fields, index, aim)
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

    old_n = nbytes(old_body)
    new_n = nbytes(new_body)
    note = f"  ({new_n / aim:.0f}x the ~{aim} aim)" if new_n > aim * 1.5 else ""
    print(f"plan.py: {name} rewritten, {old_n} -> {new_n} bytes "
          f"({end - start} -> {len(render(name, new_body))} lines){note}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
