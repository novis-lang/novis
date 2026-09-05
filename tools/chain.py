#!/usr/bin/env python3
"""The loop's goal chain, edited by a tool instead of by hand.

    python tools/chain.py                            # the order, with where the run stands
    python tools/chain.py --show 20                  # one entry, its files, its stages
    python tools/chain.py --new unix-sockets --title "a configured store is authorized by its configuring"
    python tools/chain.py --set 20 --milestone M8 --preflight docker
    python tools/chain.py --why 20 --text "Before the dossier because that entry turns the chain around."
    python tools/chain.py --move 18 --after 19
    python tools/chain.py --renumber 21 --to 22       # or --retitle 21 --to unix-sockets
    python tools/chain.py --remove 21 --delete-files
    python tools/chain.py --retire 6                 # walked: drop the acceptance list it already folded on
    python tools/chain.py --check

`docs/agent/goals/chain.toml` is the schedule the driver walks, and adding an entry to it is five
edits in four files that are easy to get half-right: three goal files nobody has a template for, a
`[[goal]]` block, a `README.md` table row, and a `plan.py --sync` for the milestone's `Carried by`
cell. This does the mechanical four and names the fifth.

**Every mutation is a text splice, never a TOML round-trip.** Half of this file is prose -- the
comment above an entry says *why the order is what it is*, which is the one thing about a chain that
cannot be re-derived -- and `tomllib` reads none of it. So an entry is parsed as (leading comment
run, `[[goal]]` block) and written back as the same lines, and everything the tool did not
deliberately change is byte-for-byte what it was. `--check` re-renders the file it just read and
says so if that is ever untrue.

Three rules are enforced rather than documented, because all three fail silently:

* **The walked prefix is frozen.** `.loop/chain.json` is an index into this list, and every switch
  has folded one walked entry's checks into the next as its floor (`goal-switch.py`). An entry at or
  before the live one that moves, is renumbered or is removed invalidates a floor that has already
  been built, and nothing downstream notices. Refused without `--force`.
* **A number is an identity, not a position.** `21-49` is deliberately free space in front of the
  dossier: `dossier.py` numbers what it appends from `max(number) + 1`, so a hand-written entry that
  takes 51 collides with a generated one. `--new` picks the next free number below 50 and `--check`
  says so when something is sitting in the emitter's range.
* **A walked entry is retired, never removed.** `--retire N` deletes goal N's `.toml` and
  `.handoff.md` and marks the entry `retired = "<date>"`; the `[[goal]]` block and the `.md` stay,
  so no position shifts and nothing that cites the prose breaks. It is refused unless every
  `[[check]]` of that goal is *provably* in the live goal already -- which is the whole safety
  argument, and the reason this is a check rather than a note in a doc. See `--retire` below.

Retirement exists because the fold is cumulative: goal 1's 80 checks are in goal 2's file, and its
267-deep descendant is the live goal today. Six walked `.toml`s were 830K of text that no tool reads
and every `grep` over `docs/` hits eight times. The driver retires each entry as it leaves it, so the
93 goals `dossier.py` appends cost that once each instead of forever.

What this deliberately does not do: rewrite the prose that *cites* a goal. `--renumber` and
`--retitle` move the files and fix the two headers that carry the number mechanically, then print
every other place the old name appears for a human to read. A tool that rewrites sentences it
cannot read is how a doc tree stops meaning anything.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import textwrap
import tomllib
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loop  # noqa: E402  -- same directory; the check schema has one home and it is `loop.py`

ROOT = Path(__file__).resolve().parent.parent
GOALS = ROOT / "docs" / "agent" / "goals"
CHAIN = GOALS / "chain.toml"
README = GOALS / "README.md"
STATE = ROOT / ".loop" / "chain.json"

#: The goal the driver is actually running. `--retire` proves a walked entry's checks are in here
#: before deleting the file they came from; every switch since that entry left has folded them
#: forward one more time, so this is where all of them end up.
LIVE_GOAL = ROOT / "docs" / "agent" / "loop-goal.toml"

#: `goal-switch.py` inserts the previous goal's whole acceptance list at this line and refuses the
#: switch outright when it is missing -- which stops a run rather than degrading it. Every scaffold
#: this tool writes carries it, and `--check` is what says a hand-written goal forgot it.
MARKER = "# <<< goal-switch: floor checks are inserted below this line >>>"

#: The first number `dossier.py --emit-goals` may take. 50 is the dossier's own entry and 21-49 is
#: the gap it left in front of itself on purpose, so inserting a hand-written goal costs one
#: `[[goal]]` block and renumbers nothing. See `goals/README.md`.
DOSSIER_NUM = 50

#: The keys a `[[goal]]` may carry, in the order they are written. `loop.py`'s `Chain._load`
#: requires the first four; `preflight` is optional and `milestone` is what `plan.py` derives every
#: `Carried by` cell from. `retired` is the date a walked entry's acceptance list was dropped, and
#: it is the one key whose PRESENCE removes two others -- a retired entry names no `toml` and no
#: `handoff`, because there are none.
KEYS = ("name", "md", "toml", "handoff", "milestone", "preflight", "retired")

WIDTH = 100

GOAL_NUM_RE = re.compile(r"^\s*(\d+)\b")
KEY_RE = re.compile(r'^(\s*)([A-Za-z_][A-Za-z0-9_]*)(\s*=\s*)"(.*)"\s*$')
HEADER = "[[goal]]"


def die(message):
    print(f"chain: {message}", file=sys.stderr)
    return 2


def rel(path):
    try:
        return Path(path).resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return Path(path).as_posix()


# ---------------------------------------------------------------------------------------------------
# The file, as text
# ---------------------------------------------------------------------------------------------------


class Entry:
    """One `[[goal]]` block plus the comment run above it, as the lines they are.

    The comment travels with the block through a move, because it is the sentence explaining why
    that goal sits where it does -- which is worthless attached to its neighbour.
    """

    def __init__(self, lead, body):
        self.lead = list(lead)      # comment lines, no blanks at either end
        self.body = list(body)      # "[[goal]]" through its last key line

    # -- reading -------------------------------------------------------------------------------

    def get(self, key, default=""):
        for line in self.body:
            m = KEY_RE.match(line)
            if m and m.group(2) == key:
                return m.group(4)
        return default

    @property
    def name(self):
        return self.get("name")

    @property
    def num(self):
        m = GOAL_NUM_RE.match(self.name)
        return int(m.group(1)) if m else None

    @property
    def slug(self):
        rest = self.name.split(None, 1)
        return rest[1] if len(rest) > 1 else ""

    @property
    def generated(self):
        """A goal `dossier.py` wrote, which is edited in the emitter and never here."""
        return ("GENERATED" in "\n".join(self.lead).upper()
                or (self.num is not None and self.num > DOSSIER_NUM))

    @property
    def retired(self):
        """The date this entry's acceptance list was dropped, or `""` while it still has one."""
        return self.get("retired")

    @property
    def files(self):
        """The keys naming a file that must be on disk -- two fewer once the entry is retired."""
        return ("md",) if self.retired else ("md", "toml", "handoff")

    # -- writing -------------------------------------------------------------------------------

    def set(self, key, value):
        """Overwrite a key in place, or append it in `KEYS` order. `None` removes it."""
        for i, line in enumerate(self.body):
            m = KEY_RE.match(line)
            if m and m.group(2) == key:
                if value is None:
                    del self.body[i]
                else:
                    self.body[i] = f'{m.group(1)}{key}{m.group(3)}"{value}"'
                return
        if value is None:
            return
        after = KEYS[:KEYS.index(key)] if key in KEYS else KEYS
        at = len(self.body)
        for i, line in enumerate(self.body):
            m = KEY_RE.match(line)
            if m and m.group(2) in after:
                at = i + 1
        self.body.insert(at, f'{key} = "{value}"')

    def why(self, text):
        """Replace the leading comment run. `""` drops it."""
        self.lead = wrap_comment(text) if text else []

    @property
    def text(self):
        return "\n".join([*self.lead, *self.body])


def wrap_comment(text):
    """A paragraph as `# `-prefixed lines at the file's width. Blank lines survive as `#`."""
    out = []
    for para in re.split(r"\n\s*\n", text.strip()):
        if out:
            out.append("#")
        out += textwrap.wrap(" ".join(para.split()), width=WIDTH - 2,
                             initial_indent="# ", subsequent_indent="# ")
    return out


def parse(text):
    """`(head, [Entry, ...])` for a chain file.

    Everything above the first `[[goal]]` is the head, comments and all: the file's preamble and
    the first entry's own comment run are not distinguishable by shape, and the first entry is the
    one thing on the chain that never moves.
    """
    lines = text.split("\n")
    starts = [i for i, ln in enumerate(lines) if ln.strip() == HEADER]
    if not starts:
        return "\n".join(lines).rstrip("\n"), []

    leads = {}
    for s in starts[1:]:
        i = s
        while i > 0 and (not lines[i - 1].strip() or lines[i - 1].lstrip().startswith("#")):
            i -= 1
        leads[s] = i

    head = "\n".join(lines[:starts[0]]).rstrip("\n")
    entries = []
    for n, s in enumerate(starts):
        stop = leads[starts[n + 1]] if n + 1 < len(starts) else len(lines)
        lead = [ln for ln in lines[leads.get(s, s):s]]
        while lead and not lead[0].strip():
            lead.pop(0)
        while lead and not lead[-1].strip():
            lead.pop()
        body = lines[s:stop]
        while body and not body[-1].strip():
            body.pop()
        entries.append(Entry(lead, body))
    return head, entries


def render(head, entries):
    """The file back as text. Round-trips byte for byte when nothing was touched."""
    out = head.rstrip("\n") + "\n\n" if head.strip() else ""
    return out + "\n\n".join(e.text for e in entries) + "\n"


def load():
    if not CHAIN.is_file():
        raise SystemExit(die(f"{rel(CHAIN)} does not exist"))
    text = CHAIN.read_text(encoding="utf-8")
    head, entries = parse(text)
    return text, head, entries


def live_index():
    """The 0-based position of the entry the run has installed, or -1 for "nothing yet"."""
    if not STATE.is_file():
        return -1
    try:
        return int(json.loads(STATE.read_text(encoding="utf-8")).get("index", -1))
    except (ValueError, OSError):
        return -1


def find(entries, num):
    """The entry numbered `num`, by its name's leading integer -- which is what a person says."""
    hits = [e for e in entries if e.num == num]
    if not hits:
        raise SystemExit(die(f"no goal {num} on the chain -- `python tools/chain.py` lists it"))
    if len(hits) > 1:
        raise SystemExit(die(f"goal {num} appears {len(hits)} times; fix the duplicate first"))
    return hits[0]


def frozen(entries, entry, force, what):
    """Refuse a mutation of an entry the run has already walked. True when it is refused."""
    pos = entries.index(entry)
    live = live_index()
    if pos > live or force:
        return False
    where = "is the live goal" if pos == live else "has already been walked"
    die(f"goal {entry.num} ({entry.name}) {where} -- {what} it invalidates the floor "
        f"goal-switch.py already folded into the entries after it.\n"
        f"       {rel(STATE)} says the run stands at position {live + 1}. Pass --force if "
        f"the run is over or was never started.")
    return True


def landing(at, force, what):
    """Refuse a landing position at or before the live entry. True when it is refused.

    `.loop/chain.json` is a positional index, so an entry that lands in front of the live one
    renumbers the walked prefix under a running driver: `Chain.refresh` then sees a prefix that
    moved, refuses to adopt the file at all, and the run finishes on the list it started with.
    """
    live = live_index()
    if at > live or force:
        return False
    die(f"position {at + 1} is at or before the live entry (position {live + 1}) -- {what} there "
        f"shifts goals the run has already walked, and {rel(STATE)} indexes this list by position. "
        f"Pass --force if the run is over or was never started.")
    return True


def write(text, dry_run, note):
    if dry_run:
        print(f"chain: --dry-run, nothing written -- {note}")
        return 0
    CHAIN.write_text(text, encoding="utf-8", newline="\n")
    print(f"chain: {note}")
    return 0


# ---------------------------------------------------------------------------------------------------
# Reading the chain out
# ---------------------------------------------------------------------------------------------------


def cmd_list(entries, show_all):
    live = live_index()
    hand = [e for e in entries if not e.generated]
    shown = entries if show_all else hand
    print(f"chain: {rel(CHAIN)} -- {len(entries)} goal(s), "
          f"{len(entries) - len(hand)} of them generated"
          + ("" if show_all or len(hand) == len(entries) else " (--all to list those too)"))
    if live >= 0:
        print(f"       {rel(STATE)}: the run stands at position {live + 1} "
              f"({entries[live].name if live < len(entries) else '?'})")
    else:
        print(f"       {rel(STATE)}: no run has installed an entry yet")
    print()
    print(f"  {'pos':>3}  {'goal':<26} {'milestone':<12} {'preflight':<9} state")
    for i, e in enumerate(entries):
        if e not in shown:
            continue
        state = "walked" if i < live else "LIVE" if i == live else "ahead"
        if e.retired:
            state += f", retired {e.retired}"
        missing = [k for k in e.files if not (ROOT / e.get(k)).is_file()]
        if missing:
            state += f"  !! {', '.join(missing)} missing"
        print(f"  {i + 1:>3}  {e.name:<26} {e.get('milestone'):<12} "
              f"{e.get('preflight'):<9} {state}")
    print()
    print("  An entry at or before the live one is frozen: its checks are already somebody's floor.")
    print("  A retired one has had that list dropped -- it is in the live goal, not in its own file.")
    return 0


def cmd_show(entries, num):
    e = find(entries, num)
    live = live_index()
    pos = entries.index(e)
    print(f"chain: position {pos + 1} of {len(entries)} -- "
          f"{'walked' if pos < live else 'LIVE' if pos == live else 'ahead of the run'}"
          + (f", retired {e.retired} (its checks are the live goal's floor)" if e.retired else ""))
    print()
    print(e.text)
    print()
    for key in e.files:
        path = ROOT / e.get(key)
        mark = " " if path.is_file() else "!"
        head = ""
        if path.is_file():
            for line in path.read_text(encoding="utf-8").split("\n"):
                if line.strip() and not line.lstrip().startswith("#") or line.startswith("# "):
                    head = line.strip()
                    break
        print(f"  {mark} {key:<8} {rel(path)}")
        if head:
            print(f"             {head[:WIDTH - 14]}")
    toml_path = ROOT / e.get("toml")
    if toml_path.is_file():
        text = toml_path.read_text(encoding="utf-8")
        stages = []
        for m in re.finditer(r'^\s*stage\s*=\s*"([^"]*)"', text, re.M):
            if m.group(1) not in stages:
                stages.append(m.group(1))
        floor = ("marker present" if MARKER in text else
                 "NO MARKER -- goal-switch.py will refuse this entry")
        print()
        print(f"  stages   {', '.join(stages) if stages else '(no [[check]] yet)'}")
        print(f"  floor    {floor}")
    return 0


# ---------------------------------------------------------------------------------------------------
# The scaffold a new goal starts as
# ---------------------------------------------------------------------------------------------------


def span(text, key):
    """A `key = [ ... ]` list, verbatim with its comments, or None.

    Text rather than `tomllib` on purpose: the lists this copies forward -- `playbook`, `plan` --
    carry comments saying which trap each line is for, and a re-render would drop every one.
    """
    lines = text.split("\n")
    for i, line in enumerate(lines):
        if re.match(rf"^\s*{re.escape(key)}\s*=\s*\[", line):
            if line.rstrip().endswith("]"):
                return line
            for j in range(i + 1, len(lines)):
                if lines[j].strip() == "]":
                    return "\n".join(lines[i:j + 1])
            return None
    return None


def table(text, name):
    """A `[name]` table, verbatim with the comment run above it, or None."""
    lines = text.split("\n")
    for i, line in enumerate(lines):
        if line.strip() == name:
            top = i
            while top > 0 and lines[top - 1].lstrip().startswith("#"):
                top -= 1
            for j in range(i + 1, len(lines)):
                if lines[j].startswith("[") or lines[j].strip() == MARKER:
                    end = j
                    break
            else:
                end = len(lines)
            out = lines[top:end]
            # The trailing run of comments belongs to the header below, not to this table --
            # `goal-switch.py` splits a block the same way, and for the same reason.
            while out and (not out[-1].strip() or out[-1].lstrip().startswith("#")):
                out.pop()
            return "\n".join(out)
    return None


def scaffold_toml(num, slug, title, prev_toml, docker):
    """A goal's acceptance file, with its predecessor's boilerplate and its own content marked TODO.

    The split is the point: `playbook`, `plan`, `[valgrind] skip` and `[wsl]` are the same in every
    goal and are copied forward so nobody retypes them; `modules`, `rules`, `adrs` and the checks
    are this goal's whole substance and are left as questions.
    """
    prev = prev_toml.read_text(encoding="utf-8") if prev_toml and prev_toml.is_file() else ""
    carried = []
    for key in ("playbook", "plan"):
        got = span(prev, key)
        carried.append(got if got else f"{key} = []   # TODO")
    tables = []
    for name in ("[valgrind]", "[wsl]"):
        got = table(prev, name)
        if got:
            tables.append(got)
    if docker:
        got = table(prev, "[docker]")
        # The keys carry over -- the floor's containers are this goal's containers -- but the
        # comment above them says why the *previous* goal needed them, which is not a sentence to
        # inherit silently.
        keys = "\n".join(ln for ln in got.split("\n") if not ln.lstrip().startswith("#")) if got \
            else '[docker]\ncompose = "tests/db/compose.yaml"\nservices = []\nmemoize_on = []'
        tables.append("# TODO: why this goal preflights a daemon -- its own checks, its floor's, or\n"
                      "# both. The driver stops the run before the first session when it is absent.\n"
                      + keys.strip("\n"))
    return f"""# Goal {num} -- {title}
# The acceptance test, as data.
#
# `{num}-{slug}.md` is the prose. One home each.
#
# THE EXPECTED OUTPUT BELOW IS FROZEN; A FIXTURE'S *SOURCE* IS NOT.

files = [
  # TODO: every `examples/*.nvs` fixture an `exact` check below names. goal-switch.py unions the
  # previous goal's list into this one, so starting empty is fine -- but the key must exist.
]

[context]

modules = [
  # TODO: six to fifteen paths, each with the one line saying why a session opens it. This list is
  # the session's whole read budget; loop-authoring.md section 2 is how it is chosen.
]

# TODO: the ADR numbers whose one-line rule this goal lives inside.
rules = []

# TODO: the ADR numbers a session may read WHOLE -- this goal's own design, and little else.
adrs = []

# TODO: the conventions.md shapes this goal writes.
shapes = ["A commit message"]

{carried[0]}

{carried[1]}

{(chr(10) * 2).join(tables)}


{MARKER}


# ---------------------------------------------------------------------------------------------------
# Stage 2 -- the keystone. Goal prose stage 2.
# ---------------------------------------------------------------------------------------------------

# TODO: one [[check]] per stage, named for what it proves rather than what it runs. Delete this one
# if the first thing this goal proves is a fixture rather than a unit test.
[[check]]
kind = "cargo-named"
stage = "2 TODO"
name = "TODO"
args = ["test", "-p", "nvs-stdlib"]
tests = [
]
"""


def scaffold_md(num, slug, title, prev_num):
    floor = (f"Goal {prev_num}'s whole acceptance list" if prev_num
             else "The live goal's whole acceptance list")
    return f"""# Loop goal {num} — {title}

TODO: the target, in two or three sentences — what is different about the language, the runtime or
the tooling once this goal is green. Not the work; the outcome. Then one sentence on why this goal
sits where it does on the chain, which is the same sentence as the comment above its `[[goal]]`
block in [chain.toml](chain.toml).

{floor} is this goal's floor, and it is never traded.

## Stage 0 — the catch-up

TODO: what is already on disk that contradicts this goal's rule, and is therefore rewritten before
anything new is written. `Nothing. No fixture predates the rule.` is a complete answer.

## Stage 1 — the floor

{floor}, carried in verbatim by `tools/goal-switch.py`. Never traded for anything above it.

## Stage 2 — the keystone: TODO

TODO: the item list, **already grouped by file set** ([loop-authoring.md](../loop-authoring.md)
§ 7) — one numbered item per edit, each naming the file and the symbol it lands at
(`crates/<crate>/src/<module>.rs:@symbol`), so a session can open the group in one `peek.py` call.

## Standing decisions

- TODO: every tradeoff this goal will meet, decided here rather than by a session at 2am
  ([loop-authoring.md](../loop-authoring.md) § 5). Anything not pre-authorized is what makes a run
  stop.
- TODO: **this goal opens no ADR number**, or **may open ADR NNNN for one folded amendment and no
  new number** — the chain contract in [README.md](README.md) is that each goal names its slots.
- TODO: where ambiguity resolves to, so it is decided-and-recorded and never `BLOCKED`.
"""


def scaffold_handoff(num, slug, title, prev_num, next_num):
    prev = (f"Goal {prev_num}'s whole list is this goal's Stage 1 floor." if prev_num else
            "The previous goal's whole list is this goal's Stage 1 floor.")
    tail = (f"- When this goal's last check goes green the driver takes goal {next_num}.\n"
            f"  `docs/agent/goals/chain.toml` is the schedule and this does not restate it."
            if next_num else
            "- When this goal's last check goes green the driver takes the next chain entry.")
    return f"""# Handoff

## State

**Goal {num} — {title} — has just started; nothing of it has landed yet.** {prev}

TODO: what is settled before the first session — the ADR that already holds the design, the shape
of the answer, the one thing a session must not re-decide.

## Next group

**Stage 2: TODO** — one file set: TODO, TODO.

- [ ] **TODO** — `crates/<crate>/src/<module>.rs:@symbol`, and the one sentence saying what it
      must answer once this box is ticked.
- [ ] **TODO**

## Backlog

- TODO: the stages this group does not reach, each with the file set it shares, so the next
  session can tell in one line whether it is cheap to take.
{tail}
"""


def cmd_new(text, head, entries, opts):
    slug = opts.new.strip().strip("/")
    if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", slug):
        return die(f"{slug!r} is not a goal slug -- lowercase words joined by hyphens, as in "
                   f"`unix-sockets`; it becomes the filenames and the entry's name")

    # Position. Default: in front of the dossier, which is where the 21-49 gap exists for.
    if opts.after is not None:
        at = entries.index(find(entries, opts.after)) + 1
    elif opts.before is not None:
        at = entries.index(find(entries, opts.before))
    elif opts.end:
        at = len(entries)
    else:
        gen = [i for i, e in enumerate(entries) if e.num and e.num >= DOSSIER_NUM]
        at = gen[0] if gen else len(entries)
    if landing(at, opts.force, "inserting"):
        return 2

    # Number. Default: the next free one below the emitter's range.
    taken = {e.num for e in entries if e.num is not None}
    if opts.number is not None:
        num = opts.number
        if num in taken:
            return die(f"goal {num} already exists -- a number is an identity and is never reused")
    else:
        below = [n for n in taken if n < DOSSIER_NUM]
        num = max(below) + 1 if below else 1
        if num >= DOSSIER_NUM:
            return die(f"the {DOSSIER_NUM - 1}-and-below range is full, and {DOSSIER_NUM} onward "
                       f"belongs to `dossier.py --emit-goals`. Renumber the dossier's own entry "
                       f"upward before adding another hand-written goal.")
    if num >= DOSSIER_NUM:
        print(f"chain: warning -- {num} is in `dossier.py`'s numbering range ({DOSSIER_NUM}+); a "
              f"generated goal may collide with it.")

    prev = entries[at - 1] if at > 0 else None
    nxt = entries[at] if at < len(entries) else None
    title = opts.title or f"TODO ({slug})"
    milestone = opts.milestone or (prev.get("milestone") if prev else "post-parity")
    preflight = opts.preflight if opts.preflight is not None else (
        prev.get("preflight") if prev else "")
    if preflight == "none":
        preflight = ""

    stem = f"{num}-{slug}"
    paths = {
        "md": GOALS / f"{stem}.md",
        "toml": GOALS / f"{stem}.toml",
        "handoff": GOALS / f"{stem}.handoff.md",
    }
    existing = [rel(p) for p in paths.values() if p.exists()]
    if existing:
        return die(f"refusing to overwrite: {', '.join(existing)}")

    entry = Entry([], [HEADER])
    entry.set("name", f"{num} {slug}")
    for key, path in paths.items():
        entry.set(key, rel(path))
    entry.set("milestone", milestone)
    if preflight:
        entry.set("preflight", preflight)
    entry.why(opts.text or
              f"TODO: why this goal sits here rather than anywhere else on the chain. The order is "
              f"a dependency chain, not a preference, and this comment is the only home of the "
              f"reason -- goal {num} ({title}).")

    fresh = list(entries)
    fresh.insert(at, entry)
    out = render(head, fresh)

    files = {
        paths["md"]: scaffold_md(num, slug, title, prev.num if prev else None),
        paths["toml"]: scaffold_toml(num, slug, title,
                                     ROOT / prev.get("toml") if prev else None,
                                     preflight == "docker"),
        paths["handoff"]: scaffold_handoff(num, slug, title, prev.num if prev else None,
                                           nxt.num if nxt else None),
    }
    if opts.dry_run:
        print(f"chain: --dry-run -- would insert goal {num} at position {at + 1} and write "
              f"{', '.join(rel(p) for p in files)}")
        print()
        print(entry.text)
        return 0
    for path, body in files.items():
        path.write_text(body, encoding="utf-8", newline="\n")
    CHAIN.write_text(out, encoding="utf-8", newline="\n")

    print(f"chain: goal {num} {slug} inserted at position {at + 1} of {len(fresh)}"
          + (f", between {prev.name} and {nxt.name}" if prev and nxt else ""))
    for path in files:
        print(f"       wrote {rel(path)}")
    print()
    print("Next, in this order:")
    print(f"  1. Fill the TODOs in {rel(paths['md'])} -- the target, the stages, the standing")
    print("     decisions. loop-authoring.md is how; the goal prose is what the TOML is derived from.")
    print(f"  2. Fill {rel(paths['toml'])}: the `[context]` manifest first (it is the session's")
    print("     whole read budget), then one `[[check]]` per stage. Leave the marker line alone.")
    print(f"  3. Add its row to {rel(README)}:")
    print(f"       | [{num} {slug}]({stem}.md) | {milestone or 'TODO'} | TODO: the crates it opens |")
    print("  4. `python tools/plan.py --sync` -- the milestone's `Carried by` cell is derived from")
    print("     this file and drifts the moment an entry lands.")
    print("  5. `python tools/chain.py --check`")
    if live_index() >= 0:
        print()
        print("The driver adopts an entry inserted ahead of the live goal without a restart "
              "(`Chain.refresh`).")
    return 0


# ---------------------------------------------------------------------------------------------------
# Editing what is there
# ---------------------------------------------------------------------------------------------------


def cmd_set(text, head, entries, opts):
    e = find(entries, opts.set)
    if frozen(entries, e, opts.force, "editing"):
        return 2
    changed = []
    for key, value in (("milestone", opts.milestone), ("preflight", opts.preflight)):
        if value is None:
            continue
        was = e.get(key)
        new = "" if value == "none" else value
        if was == new:
            continue
        e.set(key, new or None)
        changed.append(f"{key}: {was or '(none)'} -> {new or '(none)'}")
    if opts.text:
        e.why(opts.text)
        changed.append("the comment above it")
    if not changed:
        return die("nothing to change -- pass --milestone, --preflight or --text")
    rc = write(render(head, entries), opts.dry_run,
               f"goal {e.num} ({e.name}): " + "; ".join(changed))
    if opts.milestone is not None and not opts.dry_run:
        print("       `python tools/plan.py --sync` -- the milestone's `Carried by` cell follows "
              "this key.")
    return rc


def cmd_why(text, head, entries, opts):
    e = find(entries, opts.why_of)
    if frozen(entries, e, opts.force, "rewriting the comment above"):
        return 2
    body = opts.text
    if opts.from_file:
        body = Path(opts.from_file).read_text(encoding="utf-8")
    if body is None:
        print("\n".join(e.lead) if e.lead else f"chain: goal {e.num} carries no comment")
        return 0
    e.why(body)
    return write(render(head, entries), opts.dry_run,
                 f"goal {e.num} ({e.name}): comment rewritten")


def cmd_move(text, head, entries, opts):
    e = find(entries, opts.move)
    if frozen(entries, e, opts.force, "moving"):
        return 2
    if opts.after is None and opts.before is None:
        return die("--move needs --after N or --before N")
    anchor = find(entries, opts.after if opts.after is not None else opts.before)
    if anchor is e:
        return die("an entry cannot be moved relative to itself")
    fresh = [x for x in entries if x is not e]
    at = fresh.index(anchor) + (1 if opts.after is not None else 0)
    if landing(at, opts.force, "landing an entry"):
        return 2
    fresh.insert(at, e)
    if [x.name for x in fresh] == [x.name for x in entries]:
        return die(f"goal {e.num} is already there")
    rc = write(render(head, fresh), opts.dry_run,
               f"goal {e.num} ({e.name}) moved to position {fresh.index(e) + 1} of {len(fresh)}")
    if not opts.dry_run:
        print("       The comment above it travelled with it, and it is now probably wrong: the "
              "order is a dependency chain, so say why it moved (`--why`).")
    return rc


def cmd_remove(text, head, entries, opts):
    e = find(entries, opts.remove)
    if frozen(entries, e, opts.force, "removing"):
        return 2
    fresh = [x for x in entries if x is not e]
    files = [ROOT / e.get(k) for k in ("md", "toml", "handoff")]
    if opts.dry_run:
        print(f"chain: --dry-run -- would drop goal {e.num} ({e.name})"
              + (f" and delete {', '.join(rel(p) for p in files)}" if opts.delete_files else
                 f" (its three files stay on disk; --delete-files removes them)"))
        return 0
    CHAIN.write_text(render(head, fresh), encoding="utf-8", newline="\n")
    print(f"chain: goal {e.num} ({e.name}) dropped from the chain")
    if opts.delete_files:
        for path in files:
            if path.is_file():
                path.unlink()
                print(f"       deleted {rel(path)}")
    else:
        print("       its three files are still on disk -- --delete-files removes them")
    print(f"       Check {rel(README)} for a row that now names nothing, and run "
          f"`python tools/plan.py --sync`: a `Carried by` cell went with it.")
    return 0


def check_ids(path):
    """Every `[[check]]` in a goal TOML, as `(kind, name-or-file)` -- the pair the ledger prints.

    Identity and not text, because the floor is carried verbatim and then *lived in*: a session may
    raise a `min_passing`, add a test name to a `cargo-named` block or fix a fixture path, and all
    three are legitimate. What may never happen is a check going missing, which is exactly what a
    set difference over this key sees and a text comparison would drown in noise.
    """
    spec = tomllib.loads(path.read_text(encoding="utf-8"))
    return {(c.get("kind", "?"), c.get("name") or c.get("file") or "?")
            for c in spec.get("check", [])}


def cmd_retire(text, head, entries, opts):
    """Drop a walked entry's acceptance list, keeping its block and its prose.

    The deletion is safe for one reason and it is checked rather than asserted: `goal-switch.py`
    folded every `[[check]]` of this goal into the next one at the switch, and each switch since
    folded that forward again, so the live goal holds them all. This proves that before unlinking
    anything, and the proof is not `--force`-able -- a floor that has gone missing is the one thing
    retirement must never hide, and `--force` is for a run whose position bookkeeping is stale.
    """
    # Every other command here is run by a person who reads the diff. This one is run by the driver
    # at every switch, so a file that does not round-trip would be reformatted unreviewed, in a
    # commit nobody opened. Refuse instead, and let `--check` say what is wrong with it.
    if render(head, entries) != text:
        return die(f"{rel(CHAIN)} does not round-trip through this tool's parser, so retiring an "
                   f"entry would reformat lines nobody touched -- `--check` says what is wrong")
    e = find(entries, opts.retire)
    pos, live = entries.index(e), live_index()
    if e.retired:
        return die(f"goal {e.num} ({e.name}) was already retired on {e.retired}")
    if pos >= live and not opts.force:
        where = ("is the live goal -- its `.toml` is the file the driver runs" if pos == live else
                 "is ahead of the run" if live >= 0 else
                 f"cannot be retired: {rel(STATE)} says no run has installed an entry")
        return die(f"goal {e.num} ({e.name}) {where}. Only an entry the run has LEFT may be "
                   f"retired, because leaving it is what folded its checks forward.\n"
                   f"       Pass --force if the run is over and this position is stale.")

    toml_path, live_path = ROOT / e.get("toml", "x"), LIVE_GOAL
    if not toml_path.is_file():
        return die(f"goal {e.num} names {e.get('toml')}, which is not on disk -- nothing to prove "
                   f"a fold against. Fix the entry before retiring it.")
    if not live_path.is_file():
        return die(f"{rel(live_path)} does not exist, so there is nothing to prove the fold into")
    lost = sorted(check_ids(toml_path) - check_ids(live_path))
    if lost:
        die(f"goal {e.num} ({e.name}) holds {len(lost)} check(s) that {rel(live_path)} does not, "
            f"so its acceptance list was NOT folded all the way forward:")
        for kind, name in lost[:12]:
            print(f"       [{kind}] {name}", file=sys.stderr)
        if len(lost) > 12:
            print(f"       ... and {len(lost) - 12} more", file=sys.stderr)
        print("       Retiring it would delete a floor nothing else carries. Refusing.",
              file=sys.stderr)
        return 2

    victims = [ROOT / e.get(k) for k in ("toml", "handoff")]
    freed = sum(p.stat().st_size for p in victims if p.is_file())
    if opts.dry_run:
        print(f"chain: --dry-run -- would retire goal {e.num} ({e.name}): every one of its "
              f"{len(check_ids(toml_path))} distinct checks is in {rel(live_path)}, so "
              f"{', '.join(rel(p) for p in victims)} ({freed // 1024}K) would go")
        return 0

    e.set("toml", None)
    e.set("handoff", None)
    e.set("retired", date.today().isoformat())
    CHAIN.write_text(render(head, entries), encoding="utf-8", newline="\n")
    print(f"chain: goal {e.num} ({e.name}) retired -- its whole acceptance list is in "
          f"{rel(live_path)}")
    for path in victims:
        if path.is_file():
            path.unlink()
            print(f"       deleted {rel(path)}")
    print(f"       {freed // 1024}K freed. {rel(ROOT / e.get('md'))} stays: {rel(README)}, the "
          f"plan and the milestone files cite it.")
    return 0


def cmd_rename(text, head, entries, opts):
    """`--renumber N --to M` and `--retitle N --to slug`: the same move, two halves of one name."""
    num = opts.renumber if opts.renumber is not None else opts.retitle
    e = find(entries, num)
    if frozen(entries, e, opts.force, "renaming"):
        return 2
    new_num, new_slug = e.num, e.slug
    if opts.renumber is not None:
        try:
            new_num = int(opts.to)
        except ValueError:
            return die(f"--renumber --to takes an integer, not {opts.to!r}")
        if new_num in {x.num for x in entries if x is not e}:
            return die(f"goal {new_num} already exists")
        if new_num >= DOSSIER_NUM:
            print(f"chain: warning -- {new_num} is in `dossier.py`'s range ({DOSSIER_NUM}+)")
    else:
        new_slug = opts.to.strip()
        if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", new_slug):
            return die(f"{new_slug!r} is not a goal slug -- lowercase words joined by hyphens")
    old_stem, new_stem = f"{e.num}-{e.slug}", f"{new_num}-{new_slug}"
    if old_stem == new_stem:
        return die("that is the name it already has")

    moves = []
    for key, suffix in (("md", ".md"), ("toml", ".toml"), ("handoff", ".handoff.md")):
        src = ROOT / e.get(key)
        dst = GOALS / f"{new_stem}{suffix}"
        if dst.exists():
            return die(f"refusing to overwrite {rel(dst)}")
        moves.append((key, src, dst))

    if opts.dry_run:
        print(f"chain: --dry-run -- would rename goal {old_stem} -> {new_stem}")
        for _, src, dst in moves:
            print(f"       {rel(src)} -> {rel(dst)}")
        return 0

    for key, src, dst in moves:
        if src.is_file():
            body = src.read_text(encoding="utf-8")
            # The two headers that carry the number mechanically. Everything else is prose.
            body = re.sub(r"^# Loop goal \d+ —", f"# Loop goal {new_num} —", body, count=1,
                          flags=re.M)
            body = re.sub(r"^# Goal \d+ --", f"# Goal {new_num} --", body, count=1, flags=re.M)
            body = body.replace(f"{old_stem}.md", f"{new_stem}.md")
            dst.write_text(body, encoding="utf-8", newline="\n")
            src.unlink()
        e.set(key, rel(dst))
    e.set("name", f"{new_num} {new_slug}")
    CHAIN.write_text(render(head, entries), encoding="utf-8", newline="\n")
    print(f"chain: goal {old_stem} -> {new_stem}; three files moved, two headers rewritten")

    cites = references(old_stem, e.num)
    if cites:
        print()
        print(f"       These still say {old_stem!r} or `goal {e.num}` and this tool does not "
              f"rewrite prose it cannot read:")
        for path, n in cites:
            print(f"         {path}: {n} line(s)")
    print(f"       `python tools/plan.py --sync` -- `Carried by` cells hold the number.")
    return 0


def references(stem, num):
    """Every tracked file still naming the old goal, as (path, hits). Reported, never rewritten."""
    pattern = rf"{re.escape(stem)}|goal {num}\b|goals {num}\b"
    try:
        out = subprocess.run(["git", "grep", "-cEi", pattern, "--", "docs", "tools", "AGENTS.md"],
                             cwd=ROOT, capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.SubprocessError):
        return []
    hits = []
    for line in out.stdout.splitlines():
        if ":" in line:
            path, _, count = line.rpartition(":")
            hits.append((path, count))
    return hits


# ---------------------------------------------------------------------------------------------------
# --check
# ---------------------------------------------------------------------------------------------------


def cmd_check(text, head, entries):
    problems, notes = [], []

    if render(head, entries) != text:
        problems.append(
            f"{rel(CHAIN)} does not round-trip through this tool's parser, so an edit here could "
            f"reformat lines it did not mean to touch. Likely cause: an entry's keys are separated "
            f"by a blank line, or a `[[goal]]` header is indented.")

    try:
        tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        problems.append(f"{rel(CHAIN)} does not parse as TOML: {exc}")

    seen_nums, last_num = {}, 0
    live = live_index()
    for i, e in enumerate(entries):
        where = f"position {i + 1} ({e.name or 'unnamed'})"
        if e.num is None:
            problems.append(f"{where}: the name does not start with a number, and the number is "
                            f"what every other file calls this goal by")
        else:
            if e.num in seen_nums:
                problems.append(f"{where}: goal {e.num} is also at position {seen_nums[e.num]}")
            seen_nums[e.num] = i + 1
            if e.num < last_num:
                notes.append(f"{where}: numbered below the entry before it ({last_num}) -- legal, "
                             f"since a number is an identity, but `--list` reads oddly")
            last_num = e.num
            if e.num > DOSSIER_NUM and not e.generated:
                notes.append(f"{where}: sits in `dossier.py`'s numbering range ({DOSSIER_NUM}+), "
                             f"where an emitted goal may collide with it")
        if not e.get("milestone"):
            problems.append(f"{where}: names no milestone -- `plan.py --check` derives every "
                            f"`Carried by` cell from that key")
        for key in ("name", *e.files):
            if not e.get(key):
                problems.append(f"{where}: has no `{key}` -- loop.py refuses the whole chain")
        if e.retired:
            # Both halves, because either one alone is a lie the driver would act on: an entry that
            # still names a deleted file stops the chain loading, and one marked retired while its
            # list is still on disk is a floor nobody folded pretending it was folded.
            for key, suffix in (("toml", "toml"), ("handoff", "handoff.md")):
                if e.get(key):
                    problems.append(f"{where}: is retired but still names `{key}` -- a retired "
                                    f"entry has no acceptance list and no seed")
                elif (GOALS / f"{e.num}-{e.slug}.{suffix}").is_file():
                    notes.append(f"{where}: retired, but {e.num}-{e.slug}.{suffix} is still on "
                                 f"disk -- `--retire` deletes it and something put it back")
            # `live` is -1 in a tree with no `.loop/chain.json` at all, which is every fresh clone
            # and every CI job: a retired entry there is history, not a contradiction.
            if live >= 0 and i >= live:
                problems.append(f"{where}: is retired but the run stands at position {live + 1} -- "
                                f"only an entry the run has LEFT may be retired, since retiring is "
                                f"what says its checks are already somebody's floor")
        for key in e.files:
            path = ROOT / e.get(key, "x")
            if not path.is_file():
                problems.append(f"{where}: names {e.get(key)}, which does not exist")
                continue
            body = path.read_text(encoding="utf-8")
            if key == "toml":
                if MARKER not in body:
                    problems.append(f"{rel(path)}: no goal-switch marker line, so the switch into "
                                    f"this goal refuses and the run stops. Add:\n      {MARKER}")
                for need, why in (("files", "goal-switch.py unions the previous goal's fixtures "
                                            "into it and refuses when the key is absent"),
                                  ("skip", "`[valgrind] skip` is unioned the same way")):
                    if not re.search(rf"^\s*{need}\s*=\s*\[", body, re.M):
                        problems.append(f"{rel(path)}: no `{need} = [...]` -- {why}")
                if "[[check]]" not in body:
                    problems.append(f"{rel(path)}: holds no `[[check]]`, so nothing can turn it "
                                    f"green and the run stalls on it")
                # The list has to be one the DRIVER can run, not just one that greps right.
                # Everything above is a text search; this is `loop.py`'s own schema, so a
                # misspelled key is a problem printed here rather than a dead run on the night the
                # chain reaches the entry.
                fail = loop.spec_error(path)
                if fail:
                    problems.append(f"{rel(path)}: the driver cannot run this list -- {fail}")
            if key == "md" and e.num is not None:
                h1 = body.split("\n", 1)[0]
                if not h1.startswith(f"# Loop goal {e.num} "):
                    notes.append(f"{rel(path)}: its H1 is {h1[:60]!r}, which `plan.py`'s "
                                 f"`live_goal()` matches the live copy against")
            if key == "handoff" and not body.startswith("# Handoff"):
                notes.append(f"{rel(path)}: does not start with `# Handoff`")
            if "TODO" in body:
                notes.append(f"{rel(path)}: still carries TODO markers -- a scaffold nobody has "
                             f"filled in yet")
        if i > live and not e.lead and not e.generated:
            notes.append(f"{where}: no comment above it. The order is a dependency chain and this "
                         f"is the only home of the reason -- `--why {e.num} --text '...'`")

    if README.is_file():
        readme = README.read_text(encoding="utf-8")
        for e in entries:
            if e.generated:
                continue
            name = Path(e.get("md", "")).name
            if name and name not in readme:
                notes.append(f"{rel(README)}: no row for {e.name} -- the table there is what a "
                             f"reader reads instead of the TOML")

    for label, items in (("", problems), ("note: ", notes)):
        for item in items:
            print(f"  {label}{item}")
    if problems:
        print()
        print(f"chain: {len(problems)} problem(s), {len(notes)} note(s) over "
              f"{len(entries)} entries")
        return 1
    print(f"chain: {len(entries)} entries, all walkable"
          + (f" -- {len(notes)} note(s) above" if notes else ""))
    print("       `python tools/plan.py --check` is the other half: milestone tags and "
          "`Carried by` cells.")
    return 0


# ---------------------------------------------------------------------------------------------------


def main(argv=None):
    p = argparse.ArgumentParser(
        prog="chain.py",
        description=__doc__.split("\n\n")[0],
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__[__doc__.index("`docs/agent/goals/chain.toml`"):],
    )
    p.add_argument("--all", action="store_true",
                   help="list the generated goals too (default: hand-written entries only)")
    p.add_argument("--show", type=int, metavar="N", help="one entry, its files and its stages")
    p.add_argument("--new", metavar="SLUG",
                   help="scaffold a goal (three files) and insert it; defaults to the next free "
                        "number below the dossier and the position in front of it")
    p.add_argument("--set", type=int, metavar="N", help="edit goal N's keys")
    p.add_argument("--why", dest="why_of", type=int, metavar="N", nargs="?", const=-1,
                   help="print or rewrite the comment above goal N (with --text or --from)")
    p.add_argument("--move", type=int, metavar="N", help="reorder goal N (with --after/--before)")
    p.add_argument("--remove", type=int, metavar="N", help="drop goal N from the chain")
    p.add_argument("--retire", type=int, metavar="N",
                   help="walked goal N: delete its .toml and .handoff.md, keep its block and .md")
    p.add_argument("--renumber", type=int, metavar="N", help="give goal N a new number (--to M)")
    p.add_argument("--retitle", type=int, metavar="N", help="give goal N a new slug (--to SLUG)")
    p.add_argument("--check", action="store_true", help="every entry is one the driver can walk")

    p.add_argument("--title", help="--new: the goal's one-line title, as its H1 reads")
    p.add_argument("--number", type=int, help="--new: take this number instead of the next free one")
    p.add_argument("--milestone", help="the milestone tag this goal carries (M7, post-parity, ...)")
    p.add_argument("--preflight", help="`docker`, or `none` to drop the key")
    p.add_argument("--after", type=int, metavar="N", help="position: directly after goal N")
    p.add_argument("--before", type=int, metavar="N", help="position: directly before goal N")
    p.add_argument("--end", action="store_true", help="position: on the end of the chain")
    p.add_argument("--to", help="--renumber/--retitle: the new number or slug")
    p.add_argument("--text", help="the comment above the entry, as one paragraph")
    p.add_argument("--from", dest="from_file", metavar="FILE",
                   help="--why: read the comment from a file instead")
    p.add_argument("--delete-files", action="store_true",
                   help="--remove: delete the goal's three files as well")
    p.add_argument("--force", action="store_true",
                   help="edit an entry the run has already walked (invalidates a folded floor)")
    p.add_argument("--dry-run", action="store_true", help="say what would change, write nothing")
    opts = p.parse_args(argv)
    # A goal's title is prose and carries em dashes; the consoles here are cp1252.
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", newline="\n", errors="replace")

    text, head, entries = load()
    if not entries:
        return die(f"{rel(CHAIN)} holds no [[goal]] entry")

    if opts.check:
        return cmd_check(text, head, entries)
    if opts.show is not None:
        return cmd_show(entries, opts.show)
    if opts.new:
        return cmd_new(text, head, entries, opts)
    if opts.set is not None:
        return cmd_set(text, head, entries, opts)
    if opts.why_of is not None:
        if opts.why_of == -1:
            return die("--why takes a goal number")
        return cmd_why(text, head, entries, opts)
    if opts.move is not None:
        return cmd_move(text, head, entries, opts)
    if opts.remove is not None:
        return cmd_remove(text, head, entries, opts)
    if opts.retire is not None:
        return cmd_retire(text, head, entries, opts)
    if opts.renumber is not None or opts.retitle is not None:
        if not opts.to:
            return die("--renumber/--retitle needs --to")
        return cmd_rename(text, head, entries, opts)
    return cmd_list(entries, opts.all)


if __name__ == "__main__":
    raise SystemExit(main())
