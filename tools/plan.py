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
    python tools/plan.py --check                    # sizes against the aim, index against disk (CI)
    python tools/plan.py --sync                     # rewrite the derived cells from their sources
    python tools/plan.py --stale                    # sentences deferring to a goal that has walked
    python tools/plan.py --past                     # each milestone behind the program, complete or not

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
comment states and says what it costs a session; a size never changes its exit status, for the
reason docs/agent/doc-style.md gives about length tripwires. A number is a report to weigh, not a
gate.

**It does refuse over a structure**, and that is the whole of what it gates on: a row that does not
parse, a row naming a file that is not there, a title that has drifted from the H1 it was copied
from, a milestone file no row names, a milestone with no `**Verify:**`. Those are the findings a
machine can be certain about, so `--check` exits 1 on any of them and CI's `docs` job runs it. The
two kinds print in the same report and only one of them decides the exit status.

`--sync` is the other half of that: **two of the four cells are derived**, so drift is fixed by
regenerating rather than by hand-editing whichever copy the reader noticed first. The title comes
from the milestone file's own H1. The `Carried by` cell comes from `docs/agent/goals/`, which is
the chain, because **the chain is the schedule and a milestone is an identity tag one or more goals
carry** -- AGENTS.md's *The schedule is the chain* bullet is the one home of that rule, and it is
also why the cell names each goal by its slug. A milestone the program has walked past and finished
is the exception, and `done` is what its cell says: the goals that carried it have walked, so
naming them would say where the work was rather than where it is, and `--past` below is what
decides that. Loop-days is the index's own data and `--sync` carries it through untouched; so is
the cell of a milestone neither answer reaches, which has nothing to derive it from and says
`ongoing` or `backlog N` on its own.

`--stale` is a **lint over the prose**, and the only mode here that reads the milestone files for
what they say rather than for how big they are. It reports one shape: a sentence that names a goal
by slug and is still written in the future tense, when the chain has already walked that goal. That
is how a plan file goes stale -- the goal ran, the work landed, and the sentence promising it did
not move. It finds neither every stale sentence nor only stale ones, so a finding is a place to
re-read against the tree rather than a line to delete, and it exits 0 whatever it finds. The count
on its last line is what an acceptance check matches.

`--past` answers the one question the table cannot: **is a milestone the program has already walked
past actually finished?** Everything before M9 is complete at the end of this program, so a row
before it is either done or owed, and two facts say which -- every goal carrying it has walked, and
no register still tags an item to it. Both are read: the chain for the first, `tools/owners.py
--json` for the second, summed across every register it walks, since a milestone is not finished
while a ratchet key or a module doc's gap still names it. It writes nothing and exits 0
whatever it finds: `--sync` is what writes `done` into a complete one's cell, and `--check` is
what refuses that cell on a milestone this does not call complete. The three read one function,
so the table can neither claim a milestone finished early nor go on naming goals for a finished
one.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import goals as goalsmod  # noqa: E402  -- the chain's one reader

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "implementation-plan.md"
PLAN_DIR = ROOT / "docs" / "plan"
DESIGN = PLAN_DIR / "design.md"
GOALS = goalsmod.GOALS
LIVE_GOAL = ROOT / "docs" / "agent" / "loop-goal.md"


WIDTH = 100  # including the "> " prefix, matching what is already in the file
FIELD_RE = re.compile(r"^> \*\*([^*:]+):\*\*\s*(.*)$")

#: An index row: `| goal `core-depth` | [M4S](plan/m4s.md) | The `Core` API contract … | ~1.5 |`
#: The leading **carried by** cell names, by slug, the chain goals that do this milestone's work,
#: and a milestone's number is its identity rather than its position (implementation-plan.md says
#: so where the table is). It is optional here so a table written before that column, or a row that
#: never gets one, still parses -- and it is matched rather than skipped so `--check` can compare
#: it against the goals directory instead of reporting only the roster.
#: The last cell is the loop-day projection (docs/plan/velocity.md owns what it means) and is
#: optional too. Both are matched separately rather than swept into the title, because `--check`
#: compares the title against the milestone file's H1 character for character and would otherwise
#: report every row as drifted.
ROW_RE = re.compile(
    r"^\|(?:\s*([^|\[]*?)\s*\|)?\s*\[(M\d+[A-Z]?)\]\((plan/[^)]+)\)"
    r"\s*\|\s*([^|]*?)\s*\|(?:\s*([^|]*?)\s*\|)?\s*$"
)

#: The milestone table's header, and its `|---|` rule. Rows are read strictly between the header and
#: the first line that is not a table row: a `|` line inside that span which ROW_RE does not match is
#: *reported* rather than skipped. A row that silently fails to parse drops a milestone out of the
#: roster with nothing to show for it -- `787dd992` was exactly that, an added Order column that made
#: every row unparseable at once, and what found it was a human noticing the table had gone empty.
#: `Order` is the pre-chain name of the first column and is still accepted, so a tree mid-rename
#: parses rather than reporting sixteen unparsed rows at once.
TABLE_HEAD_RE = re.compile(r"^\|\s*(?:Carried by|Order)\s*\|\s*Milestone\s*\|")
TABLE_RULE_RE = re.compile(r"^\|[\s:|-]+\|$")

#: A milestone file's H1: `# M4S — The `Core` API contract and its pure half (~5 weeks)`
H1_RE = re.compile(r"^#\s+(M\d+[A-Z]?)\s*—\s*(.*)$")

#: Fallback if the plan's leading comment stops stating one. That comment is the aim's one home.
FIELD_AIM_FALLBACK = 400

#: Fallback multiple for the ceiling, likewise read from that comment. The ceiling is the one
#: size that IS enforced -- `session.py --wrap` refuses an edit that leaves a field both over it
#: and bigger than it was -- and it is a gate on growth rather than on size on purpose: a field
#: already over it can always be shrunk or held, so there is never prose to shave to clear it.
FIELD_CEILING_X_FALLBACK = 5


def h1_of(text):
    """A goal file's H1, with its front matter stripped off first.

    The milestone sits in front matter above the H1, so the first line of a goal file is `---` and
    not its title. Everything that identifies a goal by its H1 -- and `live_goal()` below is the
    only thing that does -- goes through this rather than through `split("\\n", 1)[0]`."""
    return goalsmod.FRONT_RE.sub("", text, count=1).split("\n", 1)[0].strip()


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


def field_ceiling(text=None):
    """The per-field byte ceiling a growing edit may not cross, from the same comment."""
    if text is None:
        text = PLAN.read_text(encoding="utf-8")
    m = re.search(r"over (\d+)x that", text)
    return field_aim(text) * (int(m.group(1)) if m else FIELD_CEILING_X_FALLBACK)


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
                    "carried": (m.group(1) or "").strip(),
                    "loop_days": (m.group(5) or "").strip(),
                    "path": ROOT / "docs" / m.group(3),
                    "rel": "docs/" + m.group(3),
                    "line": i + 1,
                }
            )
    return found


# ---------------------------------------------------------------------------- the chain

#: What this chain writes for work that lands in no milestone at all -- goals `temp-sweep` through `doc-comments` are five of
#: them. A milestone is not invented to hold a goal; the goal says so and `--check` accepts it.
#: It is a convention rather than the only legal word: `dossier.py --emit-goals` writes chains
#: tagged `dossier`, and a future program will have its own. Hence MILESTONE_TAG_RE below --
#: **what `--check` gates on is the shape**, so a tag spelled like a milestone must be a real row
#: (`M18` is a typo worth catching) and a tag that is plainly a label is taken as one.
POST_PARITY = "post-parity"

#: A chain tag that claims to be a milestone id. Anything matching this must be in the table. The
#: number is captured because a milestone's place in the program is its number and not its suffix:
#: `M4S` and `M4B` are M4's, which is what the table means by writing them that way, and `--past`
#: asks that question of every row.
MILESTONE_TAG_RE = re.compile(r"^M(\d+)[A-Z]?$")

#: `1 core-depth` -> 1. The number is what a person says out loud ("goal `parses`"), and it is read off
#: the entry's own name rather than its position, so inserting a goal cannot silently renumber the
#: cells of every milestone after it.
GOAL_NUM_RE = re.compile(r"^\s*(\d+)\b")

#: The cell of a milestone the chain does not carry: finished, running forever, or waiting with a
#: place in the queue behind the chain. There is nothing to derive it from, so it is the index's
#: own data -- and anything else in that cell is a `--check` finding rather than a fourth vocabulary.
UNCHAINED_CELL_RE = re.compile(r"^(?:done\\?\*?|ongoing|backlog \d+)$")

#: The cell of a milestone that is finished, in the two spellings the table writes it -- the star
#: is its own footnote marker and means the same word. On a milestone the program has already
#: walked past this is the one cell `--sync` derives and `--check` gates: it is accepted there only
#: while `past_state` calls that milestone complete, and required as soon as it does.
DONE_CELL_RE = re.compile(r"^done\\?\*?$")


def chain_goals():
    """Every goal in `docs/agent/goals/`, in chain order.

    `{pos, num, slug, md, milestone, retired}` each. The chain is the schedule -- the driver walks
    that directory in numeric order, and the index's `Carried by` cells are derived from each
    goal's front-matter `milestone` -- so a tree with no goals is a tree where those cells are all
    there is, and this answers `[]` for it. `tools/goals.py` is the reader; this only reshapes."""
    return [
        {
            "pos": g.num,
            "num": g.num,
            "slug": g.slug,
            "md": goalsmod.rel(g.md),
            "milestone": g.milestone,
            "retired": g.retired,
        }
        for g in goalsmod.load()
    ]


def carried_by(goals=None):
    """milestone id -> the slugs of the goals that carry it, in chain order.

    This is the join the whole arrangement rests on: M8 is carried by `core-part-ii`, `database`
    and `test-request` among others, M7 by `server`, `request-json`, `input-shapes` and `parses`,
    and no column of one milestone per row can say either.

    **Slugs, not numbers.** A number is a position and moves the moment anything is inserted in
    front of it. This cell is regenerated by `--sync` and would survive that either way, but the
    prose around it is not regenerated, and one table spelling a goal the old way is how the old
    way comes back."""
    by = {}
    for g in chain_goals() if goals is None else goals:
        if MILESTONE_TAG_RE.match(g["milestone"]):
            by.setdefault(g["milestone"], []).append(g["slug"])
    return by


def schedule_cell(slugs):
    """`["surface"]` -> ``goal `surface```; two or more -> ``goals `a`, `b```.

    The index's cell character for character, so `--sync` writes it and `--check` compares
    against it without either having to know how the other spells one."""
    if not slugs:
        return ""
    cited = ", ".join(f"`{s}`" for s in slugs)
    return f"goal {cited}" if len(slugs) == 1 else f"goals {cited}"


def live_goal():
    """The chain entry `docs/agent/loop-goal.md` is currently a copy of, or None.  # check-links:retired

    Matched on the H1, which `goal-switch.py` copies verbatim: the live file carries no id to read
    and the H1 is the one thing the copy and its source are guaranteed to share. `.loop/chain.json`
    holds the same fact, but only on a machine that has actually run the loop. The front matter is
    stripped off both sides first -- it is the goal's milestone, not its identity."""
    if not LIVE_GOAL.exists():
        return None
    head = h1_of(LIVE_GOAL.read_text(encoding="utf-8"))
    if not head:
        return None
    for g in chain_goals():
        if not g["md"]:
            continue
        src = ROOT / g["md"]
        if src.exists() and h1_of(src.read_text(encoding="utf-8")) == head:
            return g
    return None


# ------------------------------------------------------------------ a milestone behind the program

#: How `--past` counts what is still tagged to a milestone: out of process, the way the acceptance
#: check runs it, so this file never learns what a gap looks like written down. Which sentence in
#: the tree is owed work, and what a tag on it means, is `tools/owners.py`'s fact alone.
OWNERS = ["owners.py", "--json"]


def milestone_num(mid):
    """`M4S` -> 4, `M11` -> 11, anything else -> None."""
    m = MILESTONE_TAG_RE.match(mid.strip())
    return int(m.group(1)) if m else None


def register_tags():
    """`(owner -> items every register still tags to it, the first milestone still ahead)`.

    Read out of `tools/owners.py --json` and summed across its registers, because a milestone is
    not finished while any of them still names it -- the module docs' `# Known gaps` blocks and the
    ratchets' `#` column both tag an owner, and a report that read one of them would call a
    milestone complete over the other."""
    proc = subprocess.run([sys.executable, str(Path(__file__).with_name(OWNERS[0])), *OWNERS[1:]],
                          capture_output=True, text=True, encoding="utf-8")
    if proc.returncode != 0:
        raise RuntimeError(f"tools/{OWNERS[0]} exited {proc.returncode}: "
                           f"{(proc.stderr or proc.stdout).strip().splitlines()[-1:] or ['']}")
    data = json.loads(proc.stdout)
    tally = {}
    for reg in data["registers"].values():
        for owner, n in reg["owners"].items():
            tally[owner] = tally.get(owner, 0) + n
    return tally, data["first_future_milestone"]


def goal_state(slug, walked, live):
    """`walked`, `live` or `ahead` -- where the chain stands relative to one goal."""
    if slug in walked:
        return "walked"
    return "live" if live and live["slug"] == slug else "ahead"


def past_state(index=None, goals=None):
    """Every milestone the program has already walked past, and whether it is finished.

    `({id: {complete, tagged, carried}}, the first milestone still ahead)`, in index order, where
    `carried` is one `(slug, walked|live|ahead)` per goal carrying it. A milestone at M9 or later
    is **absent rather than incomplete**: ahead of the program is not a state this answers, and
    where that line falls is `tools/owners.py`'s fact, read with the counts.

    Complete is two conditions and both are read off the tree rather than written down anywhere:
    every goal carrying the milestone has walked, and no register still tags an item to it. This
    is that rule's one home -- `--past` reports it, `--sync` writes `done` from it, and `--check`
    accepts a `done` cell only where it holds, so the three cannot drift apart."""
    index = milestones() if index is None else index
    goals = chain_goals() if goals is None else goals
    by = carried_by(goals)
    walked = walked_goals()
    live = live_goal()
    tally, first_future = register_tags()

    out = {}
    for m in index:
        num = milestone_num(m["id"])
        if num is None or num >= first_future:
            continue
        slugs = by.get(m["id"], [])
        tagged = tally.get(m["id"], 0)
        out[m["id"]] = {
            "complete": tagged == 0 and all(s in walked for s in slugs),
            "tagged": tagged,
            "carried": [(s, goal_state(s, walked, live)) for s in slugs],
        }
    return out, first_future


def run_past():
    """One line per milestone the program is already past, and whether it is complete.

    The mechanical half of the rule that M0 through M8 are finished at the end of this program;
    the judgement half is each closure goal's own acceptance list. Nothing here writes and nothing
    here exits non-zero -- a count is a report to act on, and `--sync` is what acts on it."""
    rows, first_future = past_state()
    live = live_goal()
    print(f"plan.py --past: the {len(rows)} milestone(s) the index holds before M{first_future}, "
          f"against the chain and every register tools/owners.py reads")
    print("  complete = every goal carrying it has walked, and nothing is still tagged to it"
          + (f"; live at goal `{live['slug']}`" if live else ""))

    for mid, st in rows.items():
        carried = ", ".join(f"`{slug}` {state}" for slug, state in st["carried"])
        print(textwrap.fill(
            f"{mid:<4} {'complete' if st['complete'] else 'open':<8} "
            f"carried by {carried or 'no goal on the chain'}, "
            f"{st['tagged']} item(s) still tagged to it",
            width=98, initial_indent="  ", subsequent_indent=" " * 16,
            break_on_hyphens=False, break_long_words=False))

    print("\nA count above zero names work no closure goal has taken yet: `python tools/owners.py "
          "--check --past-is-an-error` lists the items behind it, each to be closed or re-owned.")
    print(f"{sum(st['complete'] for st in rows.values())} of {len(rows)} past milestone(s) "
          f"complete")
    return 0


# ----------------------------------------------------------------------- milestones, continued


def unparsed_rows(lines=None):
    """(line number, text) for every line in the milestone table that ROW_RE did not match.

    `milestones()` collects what parsed; this collects what did not, which is the only way a caller
    can tell a table of sixteen rows and one typo from a table of sixteen rows."""
    if lines is None:
        _text, lines = load()
    bad = []
    inside = False
    for i, raw in enumerate(lines):
        if TABLE_HEAD_RE.match(raw):
            inside = True
            continue
        if not inside:
            continue
        if not raw.lstrip().startswith("|"):
            break
        if TABLE_RULE_RE.match(raw.strip()) or ROW_RE.match(raw):
            continue
        bad.append((i + 1, raw.strip()))
    return bad


def sync_titles(lines):
    """The index's title cells, rewritten from each milestone file's own H1.

    Returns the new lines and one (id, rel, was, now) per row that moved. Only the title cell is
    touched, and only where the file opens with an H1 claiming that milestone's own id -- a file
    that does not is a finding for `--check` to report, not something to write a guess over."""
    out = list(lines)
    changed = []
    for m in milestones(lines):
        if not m["path"].exists():
            continue
        first = m["path"].read_text(encoding="utf-8").split("\n")[0]
        h1 = H1_RE.match(first)
        if not h1 or h1.group(1) != m["id"]:
            continue
        want = h1.group(2).strip()
        if want == m["title"]:
            continue
        raw = out[m["line"] - 1]
        row = ROW_RE.match(raw)
        start, end = row.span(4)
        out[m["line"] - 1] = raw[:start] + want.replace("|", r"\|") + raw[end:]
        changed.append((m["id"], m["rel"], m["title"], want))
    return out, changed


def sync_schedule(lines, state=None):
    """The index's `Carried by` cells, rewritten from the goals directory and the registers.

    Returns the new lines and one (id, was, now) per row that moved. Two answers are derivable and
    the milestone's own place in the program says which one applies: a milestone the program has
    walked past and finished is `done` whatever carried it, and everything else is the goals that
    carry it now. A row neither answer reaches is left alone -- it has nothing to derive its cell
    from, and guessing `backlog` for it would be this tool inventing a schedule rather than
    reading one."""
    by = carried_by()
    if not by:
        return list(lines), []
    index = milestones(lines)
    if state is None:
        state, _first_future = past_state(index)
    out = list(lines)
    changed = []
    for m in index:
        st = state.get(m["id"])
        want = "done" if st and st["complete"] else schedule_cell(by.get(m["id"]) or [])
        if not want or m["carried"] == want:
            continue
        raw = out[m["line"] - 1]
        row = ROW_RE.match(raw)
        if not row or row.group(1) is None:
            continue
        start, end = row.span(1)
        out[m["line"] - 1] = raw[:start] + want + raw[end:]
        changed.append((m["id"], m["carried"], want))
    return out, changed


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

    goals = chain_goals()
    if goals:
        live = live_goal()
        print(f"\n{GOALS.relative_to(ROOT).as_posix()}: {len(goals)} goals -- this is the "
              "schedule, and the table's first column is derived from it")
        if live:
            print(f"  live: `{live['slug']}`, {live['num']} of {len(goals)}"
                  + (f", inside {live['milestone']}" if live["milestone"] else ""))
        for m in index:
            slugs = carried_by(goals).get(m["id"])
            if slugs:
                print(f"  {m['id']:<6} {schedule_cell(slugs)}")

    print("\n--get <field> / --set <field> --from <file> for the status block;")
    print("--show M8 / --show M8:verify / --amend M8 --from <file> for a milestone;")
    print("--check prices the block and checks the index against what is on disk.")
    print("Changing several of these at the end of a session? `python tools/session.py --wrap`")
    print("takes them all in one file, with the handoff and the commits, in one call.")


def run_check(fields, index, aim):
    """Sizes report; structure gates. Nothing here exits non-zero over a size."""
    problems = []

    total = sum(nbytes(b) for _n, _a, _b2, b in fields)
    print(f"status block: {total} bytes across {len(fields)} fields, aim ~{aim} each "
          f"(~{aim * len(fields)})")
    ceiling = field_ceiling()
    for name, _a, _b, body in fields:
        n = nbytes(body)
        if n > ceiling:
            print(f"  {name:<20} {n:>6} bytes   OVER the {ceiling} B ceiling -- an edit that "
                  f"grows it is refused until it is cut")
        elif n > aim * 1.5:
            print(f"  {name:<20} {n:>6} bytes   {n / aim:.0f}x the aim, "
                  f"{ceiling - n} B under the {ceiling} B ceiling")
    print("  Every one of these is shipped into every session by orient.py and brief.py.")
    print(f"  The aim is guidance; the {ceiling} B ceiling is a gate on GROWTH: session.py --wrap")
    print("  refuses an edit that leaves a field both over it and bigger than it was. A shrink")
    print("  is always taken, so there is never prose to shave -- a sentence is replaced instead.")

    print("\nindex vs disk:")
    for lineno, raw in unparsed_rows():
        problems.append(
            f"implementation-plan.md:{lineno}: a line in the milestone table that does not parse "
            f"as a row, so no milestone was read from it\n      {raw[:96]}"
        )
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

    # The index against the chain. Two schedules that disagree is the failure this pair of checks
    # exists to make impossible to keep: the chain is what the driver walks, the cells are what a
    # reader reads, and they are the same fact written twice on purpose (once as data, once in a
    # table) rather than two facts to reconcile by hand.
    goals = chain_goals()
    if goals:
        ids = {m["id"] for m in index}
        by = carried_by(goals)
        for g in goals:
            if not g["milestone"]:
                problems.append(
                    f"goal `{g['slug']}`: its `.md` front matter names no milestone -- give it "
                    f'one, or `milestone: {POST_PARITY}` if it lands in none'
                )
            elif MILESTONE_TAG_RE.match(g["milestone"]) and g["milestone"] not in ids:
                problems.append(
                    f"goal `{g['slug']}`: tagged {g['milestone']}, which is spelled like a "
                    "milestone id but is not a row in the milestone table"
                )
        state, _first_future = past_state(index, goals)
        for m in index:
            nums = by.get(m["id"])
            st = state.get(m["id"])
            says_done = bool(DONE_CELL_RE.match(m["carried"]))
            if st and st["complete"]:
                # `done` is what a finished milestone's cell says, and it outranks the goal list:
                # the goals that carried it have walked, so naming them says where the work was
                # rather than where it is. The second table below the index is what still names
                # them, and `--past` prints them beside the count that settles this.
                if not says_done:
                    problems.append(
                        f"{m['id']}: every goal carrying it has walked and no register still tags "
                        f"an item to it, so its cell is `done`\n"
                        f"      index: {m['carried'] or '(empty)'}\n"
                        f"      `python tools/plan.py --past` is the report, `--sync` writes it"
                    )
            elif st and says_done:
                held = [f"goal `{slug}` has not walked" for slug, where in st["carried"]
                        if where != "walked"]
                if st["tagged"]:
                    held.append(f"{st['tagged']} item(s) in the registers still name it")
                problems.append(
                    f"{m['id']}: its cell says it is finished and `python tools/plan.py --past` "
                    f"does not -- {'; '.join(held)}"
                )
            elif nums:
                want = schedule_cell(nums)
                if m["carried"] != want:
                    problems.append(
                        f"{m['id']}: the index row and the goals directory have drifted apart\n"
                        f"      index: {m['carried'] or '(empty)'}\n      chain: {want}"
                    )
            elif not UNCHAINED_CELL_RE.match(m["carried"]):
                problems.append(
                    f"{m['id']}: no chain goal carries it, so its cell says where it stands on its "
                    f"own -- `done`, `ongoing` or `backlog N`, not {m['carried'] or '(empty)'!r}"
                )
        labels = sorted({g["milestone"] for g in goals
                         if g["milestone"] and not MILESTONE_TAG_RE.match(g["milestone"])})
        pp = [f"`{g['slug']}`" for g in goals if g["milestone"] in labels]
        live = live_goal()
        print(f"  chain: {len(goals)} goals, {len(by)} milestone(s) carried"
              + (f", {len(pp)} in none (goals {', '.join(pp)}, tagged {'/'.join(labels)})"
                 if pp else "")
              + (f"; live at goal `{live['slug']}`" if live else "; nothing live"))
    else:
        print(f"  chain: no goals under {GOALS.relative_to(ROOT).as_posix()}, so the `Carried by` "
              "cells are unchecked -- they are derived from it and nothing else states them")

    if problems:
        for p in problems:
            print(f"  !! {p}")
        print(f"\n  {len(problems)} structural finding(s) -- these are what this exits non-zero "
              "on. A drifted title is `python tools/plan.py --sync`; the rest are edits.")
        return 1
    print(f"  {len(index)} rows, {len(index)} files, titles matching, every one with a "
          "`**Verify:**`, every cell agreeing with the chain")
    return 0


# ----------------------------------------------------------------------- the stale-sentence lint


#: A goal citation in the house spelling -- ``goal `surface``` or ``goals `a`, `b` and `c```.
#: AGENTS.md's *Name a goal by its slug* bullet is why there is one shape to match here: a number
#: standing where a goal's name goes is `tools/chain.py --check`'s finding, never this one's. The
#: case is ignored for the word alone, because a sentence opening on `Goal `per-core`` is the same
#: citation; the slug inside the backticks is lower case by the chain's own filename rule.
GOAL_CITE_RE = re.compile(r"\bgoals?\s+((?:`[a-z0-9][a-z0-9-]*`(?:\s*(?:,\s*and|,|and)\s*)?)+)",
                          re.IGNORECASE)
SLUG_RE = re.compile(r"`([a-z0-9][a-z0-9-]*)`")

#: The tense that turns a citation into a deferral. A sentence saying a walked goal *did*
#: something is the plan working; one saying it *will* is the shape the audit kept finding, so
#: these words are the filter that separates a promise from a reference.
FUTURE_RE = re.compile(
    r"\b(?:will|waits|until|arrives|scheduled)\b|still owed|not yet|the one that|finishes it",
    re.IGNORECASE,
)

#: A sentence boundary: terminal punctuation, whitespace, then something a sentence can open with.
#: Requiring that opener is what keeps `e.g. the` and `M4S. Part I` from splitting mid-clause.
SENTENCE_RE = re.compile(r"(?<=[.!?])\s+(?=[A-Z`*\[(\"])")


def prose_blocks(text):
    """The document as blank-line-separated blocks, each `[(line number, text)]`, fences dropped.

    Line numbers are 1-based and kept per line rather than per block, so a finding inside a wrapped
    paragraph cites the line its sentence starts on. A fenced code block is blanked rather than
    removed, because every line number after it has to stay the one in the file."""
    blocks, block, fenced = [], [], False
    for i, raw in enumerate(text.split("\n")):
        if raw.lstrip().startswith("```"):
            fenced = not fenced
            raw = ""
        elif fenced:
            raw = ""
        if raw.strip():
            block.append((i + 1, raw.strip()))
        elif block:
            blocks.append(block)
            block = []
    if block:
        blocks.append(block)
    return blocks


def block_sentences(block):
    """`[(line number, sentence)]` for one block, each sentence joined onto one line.

    The block is joined before it is split, because a sentence in this repository's prose crosses
    a wrap boundary more often than not. The line each one is charged to is found from where its
    first character landed in the join, which is why the offsets are carried alongside."""
    joined, offsets, at = [], [], 0
    for lineno, raw in block:
        offsets.append((at, lineno))
        joined.append(raw)
        at += len(raw) + 1
    text = " ".join(joined)
    out, pos = [], 0
    for part in SENTENCE_RE.split(text):
        start = text.find(part, pos)
        if start < 0:
            start = pos
        pos = start + len(part)
        lineno = offsets[0][1]
        for off, n in offsets:
            if off <= start:
                lineno = n
        out.append((lineno, part.strip()))
    return out


def walked_goals():
    """slug -> chain entry, for every goal the chain is already past.

    Two facts say a goal has walked and they agree: it sits in front of the live goal, or its
    `.toml` is gone, which is exactly what retiring one deletes. The union is taken so that a
    checkout whose `docs/agent/loop-goal.md` names nothing in the chain -- one that has never run  # check-links:retired
    the loop -- still lints against the retired half rather than against nothing."""
    live = live_goal()
    return {
        g["slug"]: g
        for g in chain_goals()
        if g["retired"] or (live and g["num"] < live["num"])
    }


def run_stale(fields):
    """Every sentence in the plan that defers work to a goal the chain has already walked.

    A lint, not a proof. It matches one shape -- a goal cited by slug in a sentence still in the
    future tense -- and reads `docs/plan/m*.md` plus the status block, whose fields are charged to
    the line their `> **Field:**` opens on because the block is stored wrapped and rewritten
    unwrapped. Nothing here exits non-zero: the count on the last line is the finding, written
    after its label so that a `want` matching `0` cannot also match `10`."""
    walked = walked_goals()
    live = live_goal()
    chain = chain_goals()
    print(f"plan.py --stale: docs/plan/m*.md and {PLAN.relative_to(ROOT).as_posix()}'s status "
          f"block, against the {len(walked)} goal(s) the chain has walked")
    if live:
        print(f"  live: `{live['slug']}`, {live['num']} of {len(chain)}")
    else:
        print("  no live goal on disk -- only the retired entries count as walked")

    sources = [
        (p.relative_to(ROOT).as_posix(), prose_blocks(p.read_text(encoding="utf-8")))
        for p in sorted(PLAN_DIR.glob("m*.md"))
    ]
    sources.append((PLAN.relative_to(ROOT).as_posix(),
                    [[(a + 1, body)] for _name, a, _b, body in fields]))

    found = 0
    for rel, blocks in sources:
        for block in blocks:
            for lineno, sentence in block_sentences(block):
                cited = [
                    s
                    for m in GOAL_CITE_RE.finditer(sentence)
                    for s in SLUG_RE.findall(m.group(1))
                    if s in walked
                ]
                marker = FUTURE_RE.search(sentence)
                if not cited or not marker:
                    continue
                found += 1
                names = ", ".join(f"`{s}`" for s in dict.fromkeys(cited))
                print(f"\n{rel}:{lineno}  {names}  -- \"{marker.group(0).lower()}\"")
                print(f"    {sentence if len(sentence) <= 160 else sentence[:159] + '…'}")

    print("\nEach is a sentence to re-read against the tree rather than one to delete: the goal has"
          " run, so either the work landed and the sentence is rewritten to what is true, or it is"
          " still owed and the sentence names the goal that owns it now.")
    print(f"sentences deferring to a walked goal: {found}")
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
    ap.add_argument("--stale", action="store_true",
                    help="sentences deferring to a goal the chain has already walked")
    ap.add_argument("--past", action="store_true",
                    help="one line per milestone behind the program, and whether it is complete")
    ap.add_argument("--sync", action="store_true",
                    help="rewrite the index's title cells from each milestone file's H1")
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

    if opts.sync:
        bad = unparsed_rows(lines)
        if bad:
            for lineno, raw in bad:
                print(f"plan.py: !! implementation-plan.md:{lineno} does not parse as a row\n"
                      f"      {raw[:96]}")
            print("plan.py: refusing to write over a table this tool cannot read whole")
            return 1
        out, changed = sync_titles(lines)
        out, moved = sync_schedule(out)
        if not changed and not moved:
            print(f"plan.py: {len(index)} rows, every title already matching its file's H1 and "
                  "every `Carried by` cell already matching the chain and the registers -- "
                  "nothing to write")
            return 0
        PLAN.write_text("\n".join(out), encoding="utf-8", newline="")
        for mid, rel, was, now in changed:
            print(f"plan.py: {mid} title synced from {rel}\n      was: {was}\n      now: {now}")
        for mid, was, now in moved:
            print(f"plan.py: {mid} carried-by synced from the chain and the registers\n"
                  f"      was: {was or '(empty)'}\n      now: {now}")
        print(f"plan.py: {len(changed) + len(moved)} cell(s) rewritten in "
              f"{PLAN.relative_to(ROOT).as_posix()}")
        return 0

    if opts.check:
        return run_check(fields, index, aim)

    if opts.stale:
        return run_stale(fields)

    if opts.past:
        return run_past()

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
