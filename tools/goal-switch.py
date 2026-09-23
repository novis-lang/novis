#!/usr/bin/env python3
"""Switch the loop to a new goal, carrying the old goal's checks in as the non-regression floor.

    python tools/goal-switch.py docs/agent/goals/<new-goal>.toml
    python tools/goal-switch.py docs/agent/goals/<new-goal>.toml --dry-run

[loop-authoring.md](../docs/agent/loop-authoring.md) § 6 makes the previous goal's whole acceptance
list the next goal's Stage 1 -- "a non-regression floor, never traded for anything above it." That is
the sentence a hand-merge gets wrong, and it gets it wrong silently: a floor that is missing looks
exactly like a floor that passes. Twenty-odd `[[check]]` blocks copied by hand at the moment a run is
being handed over is the worst possible time to be careful.

So this does it mechanically. It reads the live `docs/agent/loop-goal.toml`, takes every `[[check]]`
block out of it *verbatim as text* -- comments, formatting and all, because a check's comment says
what it guards and that is not the new goal's to rewrite -- relabels each one's `stage` to the floor
stage, and inserts them into the new goal at its marker line:

    # <<< goal-switch: floor checks are inserted below this line >>>

Two things are not carried. The previous switch's own marker and banner sit in the comment run
above the live goal's first floor check, and carried they would stack one copy deeper per goal. A
check whose spec, once relabelled, equals one already in the floor is carried once: a goal names
the conformance tree at every stage that leans on it, and at the floor stage those are one check.

The `files` lists are unioned, since a floor whose fixtures are missing fails before anything is
built. `[valgrind] skip` is unioned for the same reason. Everything else in the new goal -- its
`[context]` block, its own checks, its `[wsl]` target -- is left exactly as written: this script
carries the floor across and nothing else.

The result is written to the new goal's own path, not to `loop-goal.toml`. Renaming it is step 4 of
the switch and stays a deliberate act, because that is the point where the run's target changes.

This script judges nothing and runs nothing. It is text in, text out; `python tools/loop.py --list`
against the result is what says whether the floor arrived.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIVE = ROOT / "docs" / "agent" / "loop-goal.toml"
MARKER = "# <<< goal-switch: floor checks are inserted below this line >>>"
FLOOR_STAGE = "1 floor"


def rel_to_root(path):
    """The path as the repository names it, so a banner this writes into a committed file records
    `docs/agent/loop-goal.toml` rather than whichever absolute checkout the loop ran from."""
    resolved = Path(path).resolve()
    try:
        return resolved.relative_to(ROOT).as_posix()
    except ValueError:
        return resolved.as_posix()

# A `[[check]]` block starts at its own header line and runs to the next top-level table header or
# the end of the file. Comments immediately above a header belong to that block, not to the one
# before it -- a comment saying what a check guards is worthless attached to its neighbour.
HEADER = re.compile(r"^\[\[?[A-Za-z_][A-Za-z0-9_.\-]*\]?\]\s*$")
STAGE_LINE = re.compile(r'^(\s*stage\s*=\s*)"[^"]*"(.*)$')


def die(message):
    print(f"goal-switch: {message}", file=sys.stderr)
    return 2


def blocks(text):
    """Every top-level table in `text`, as (header, lines-including-leading-comments).

    The leading run of comment and blank lines above a header travels with it, so a block that is
    moved keeps the sentence explaining why it exists.
    """
    out, header, body = [], None, []
    for line in (ln.rstrip("\r\n") for ln in text.splitlines()):
        if not HEADER.match(line):
            body.append(line)
            continue
        # The run of comments and blanks at the tail of the block we are closing belongs to the
        # header we have just met, not to the one before it.
        lead = []
        while body and (body[-1].strip().startswith("#") or not body[-1].strip()):
            lead.insert(0, body.pop())
        while lead and not lead[0].strip():
            lead.pop(0)
        while lead and not lead[-1].strip():
            lead.pop()
        out.append((header, body))
        header, body = line, [*lead, line]
    out.append((header, body))
    return [(h, b) for h, b in out if h is not None or b]


def check_blocks(text):
    """Just the `[[check]]` blocks, in file order, as lists of lines."""
    return [lines for header, lines in blocks(text) if header == "[[check]]"]


BANNER = (
    re.compile(r"^# \d+ check\(s\) carried from .* by tools/goal-switch\.py\.$"),
    re.compile(r'^# They are the previous goal\'s acceptance list VERBATIM, relabelled to stage ".*"\.$'),
    re.compile(r"^# Do not edit them to make something pass: a floor that has been adjusted is not a floor\.$"),
)


def unbannered(lines):
    """This block's lines with any earlier switch's marker and banner taken out of its leading
    comments, and the blank lines that separated them collapsed.

    The live goal's first floor check sits directly under the marker and banner the switch before
    this one wrote, and `blocks` hands a header the whole comment run above it. Carried as they are,
    those four lines land under the new marker and banner, and the switch after that carries both --
    one more copy per goal, each banner naming a floor size that was true one switch ago. The marker
    is also load-bearing: `chain.py` and `dossier.py` test a goal for it, and a copy inside a
    comment run is one they cannot tell from the real one.
    """
    at = next(i for i, ln in enumerate(lines) if HEADER.match(ln))
    lead = [ln for ln in lines[:at]
            if ln.strip() != MARKER and not any(p.match(ln) for p in BANNER)]
    out = []
    for ln in lead:
        if ln.strip() or (out and out[-1].strip()):
            out.append(ln)
    while out and not out[0].strip():
        out.pop(0)
    while out and not out[-1].strip():
        out.pop()
    return [*out, *lines[at:]]


def dedupe(floor):
    """The floor with each check carried once: a later block whose parsed spec equals an earlier
    one's is dropped, comments and all.

    Relabelling is what makes them equal. A goal names the conformance tree once per stage that
    leans on it, and the goal before it already carried the same check at the floor stage; once
    every copy reads `stage = "1 floor"` they are one check, and the driver files them under one
    memo key. Left in, each switch carries every copy forward and the next goal adds its own, so
    the count only ever grows. The first copy keeps its comment; a later one said why a goal that is
    now behind the run leaned on the check, and the check is the same either way.
    """
    seen, out = set(), []
    for block in floor:
        key = json.dumps(tomllib.loads("\n".join(block))["check"][0], sort_keys=True)
        if key in seen:
            continue
        seen.add(key)
        out.append(block)
    return out


def relabel(lines, stage):
    """Rewrite this block's `stage = "..."`, adding one if it had none.

    A block with no stage is a real possibility -- the field is optional and the driver prints `?`
    for it -- and a floor check with no stage is a floor check the ledger cannot name.
    """
    out, seen = [], False
    for line in lines:
        m = STAGE_LINE.match(line)
        if m:
            out.append(f'{m.group(1)}"{stage}"{m.group(2)}')
            seen = True
        else:
            out.append(line)
    if not seen:
        # After the header, which is the first non-comment line.
        at = next(i for i, ln in enumerate(out) if HEADER.match(ln))
        out.insert(at + 1, f'stage = "{stage}"')
    return out


def union_list(new_text, field, extra):
    """Add every element of `extra` that `new_text`'s `field = [...]` does not already hold.

    Both list shapes this repository writes are matched, and the **single-line one is tried first**:
    its `[` and `]` sit on one line, so it is the narrower claim and the only one that can be read
    off a single line with certainty. The multi-line pattern is `.*?` under `re.S` and stops at the
    first `]` starting a line, which for a single-line `files = []` is not its own closing bracket
    at all but the next multi-line list's -- `[context] modules`, a few lines below in every goal
    file. Matched that way the union splices the carried entries into `modules`, leaves `files`
    empty, and hands the driver a floor whose every fixture check names a file `files` does not
    hold. Ordering the alternatives the other way is what makes the narrower shape unmistakable.

    A union that kept nothing and a union with nothing to add look identical from the outside, which
    is why a missing field is `None` here and a refusal in the caller rather than a quiet
    pass-through.

    `extra` is deduplicated against itself as well as against the destination. Each switch reads
    the previous goal's list back as its `extra`, so a duplicate carried once is otherwise carried
    by every switch after it and never dropped.

    Written as a text edit rather than a re-serialization on purpose: re-emitting the whole TOML
    from `tomllib`'s parse would throw away every comment in the file, and this repository's TOML is
    more comment than data.
    """
    if not extra:
        return new_text
    single = re.search(rf'^{field}\s*=\s*\[([^\[\]\n]*)\]', new_text, re.M)
    multi = None if single else re.search(rf'^{field}\s*=\s*\[(.*?)^\]', new_text, re.S | re.M)
    m = single or multi
    if not m:
        return None
    body = m.group(1)
    missing = list(dict.fromkeys(e for e in extra if f'"{e}"' not in body))
    if not missing:
        return new_text
    if multi:
        tail = "  # carried from the previous goal by tools/goal-switch.py\n"
        added = "".join(f'  "{e}",\n' for e in missing)
    else:
        tail = ""
        added = (", " if body.strip() else "") + ", ".join(f'"{e}"' for e in missing)
    return new_text[:m.end(1)] + tail + added + new_text[m.end(1):]


class CarryError(Exception):
    """Why `carry` refused: one line, said by `main` as `die` and by `tools/side.py` as its own."""


def carry(new_text, live_text, live_name, stage=FLOOR_STAGE, carried_only=False):
    """`new_text` with `live_text`'s checks inserted at its marker as the floor, and the list of
    blocks that went in. Raises `CarryError` for a goal with no marker or no list key to union into.

    `carried_only` takes only the checks `live_text` itself already carries at `stage`, which is
    the floor a side goal runs over: the live goal's own checks are unfinished work, and carried
    into a side run they would hold it red on somebody else's goal (`tools/side.py`). With it off,
    this is the chain switch -- the previous goal went green, so all of it is the floor.

    An empty floor is returned rather than refused: `main` refuses it for a switch, and a side goal
    carried into a goal whose floor is still empty has nothing to carry and nothing wrong."""
    if MARKER not in new_text:
        raise CarryError(f"it has no marker line. Add it where the floor belongs:\n    {MARKER}")
    blocks_in = check_blocks(live_text)
    if carried_only:
        blocks_in = [b for b in blocks_in
                     if any(STAGE_LINE.match(ln) and f'"{stage}"' in ln for ln in b)]
    every = [relabel(unbannered(b), stage) for b in blocks_in]
    floor = dedupe(every)
    live_spec = tomllib.loads(live_text)
    for field, carried in (("files", live_spec.get("files", [])),
                           ("skip", live_spec.get("valgrind", {}).get("skip", []))):
        merged = union_list(new_text, field, carried)
        if merged is None:
            raise CarryError(f"it has no `{field} = [...]` for the {len(carried)} "
                             f"entr{'y' if len(carried) == 1 else 'ies'} {live_name} carries. "
                             f"Add the key -- an empty list is enough -- and run this again.")
        new_text = merged
    if not floor:
        return new_text, floor, 0
    banner = (
        f"# {len(floor)} check(s) carried from {live_name} by tools/goal-switch.py.\n"
        f"# They are the previous goal's acceptance list VERBATIM, relabelled to stage "
        f'"{stage}".\n'
        "# Do not edit them to make something pass: a floor that has been adjusted is not a floor.\n"
    )
    body = banner + "\n" + "\n\n".join("\n".join(b) for b in floor) + "\n"
    return new_text.replace(MARKER, MARKER + "\n\n" + body, 1), floor, len(every) - len(floor)


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("new_goal", help="the staged goal TOML to carry the floor into")
    ap.add_argument("--live", default=str(LIVE), help="the goal being replaced")
    ap.add_argument("--stage", default=FLOOR_STAGE, help=f"stage label for the floor (default {FLOOR_STAGE!r})")
    ap.add_argument("--carried-only", action="store_true",
                    help="carry only the checks the live goal already carries at the floor stage "
                         "-- a side goal's floor (tools/side.py)")
    ap.add_argument("--dry-run", action="store_true", help="say what would change, write nothing")
    opts = ap.parse_args()

    new_path = Path(opts.new_goal)
    live_path = Path(opts.live)
    if not new_path.is_file():
        return die(f"{new_path} does not exist")
    if not live_path.is_file():
        return die(f"{live_path} does not exist -- there is no floor to carry")

    new_text = new_path.read_text(encoding="utf-8")
    live_text = live_path.read_text(encoding="utf-8")

    if not check_blocks(live_text):
        return die(f"{live_path} holds no [[check]] block -- refusing to write an empty floor")
    try:
        out, floor, dropped = carry(new_text, live_text, rel_to_root(live_path), opts.stage,
                                    opts.carried_only)
    except CarryError as e:
        return die(f"{new_path}: {e}")

    already = len(check_blocks(new_text))
    print(f"goal-switch: {len(floor)} floor check(s) from {live_path.name} "
          f"-> {new_path.name} (which already had {already})"
          + (f"; {dropped} duplicate(s) carried once" if dropped else ""))
    for b in floor:
        def field(key, default="?"):
            for line in b:
                if line.strip().startswith(f"{key} ") or line.strip().startswith(f"{key}="):
                    return line.split("=", 1)[1].strip().strip('"')
            return default
        # A program check names a fixture and a cargo one names a crate; both are how the ledger
        # will refer to it, so print whichever this block has.
        print(f"    [{opts.stage}] {field('kind'):<11} {field('name', field('file'))}")

    if opts.dry_run:
        print("goal-switch: --dry-run, nothing written")
        return 0

    new_path.write_text(out, encoding="utf-8", newline="\n")
    print(f"goal-switch: wrote {new_path}")
    print("goal-switch: next -- `python tools/loop.py --list` against it, then rename it to "
          "docs/agent/loop-goal.toml")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
