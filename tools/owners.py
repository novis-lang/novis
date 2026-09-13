#!/usr/bin/env python3
"""Every gap a module doc records, and who owns it -- read out of the docs, never copied.

A crate records what its subsystem still owes under a `# Known gaps` heading in its own module
doc. That is the right home: every fact in this repository has one, and a gap's home is the module
that owes it. What the home could not say until now is **who owns the gap**, so the whole inventory
was invisible to every tool -- nothing in the tree is shaped wrong, so no compiler, test or lint has
anything to report. `docs/agent/carried-gaps.md` indexes the handful whose ownership needed an
argument; the rest were an unindexed list nobody could count.

**The owner lives with the gap.** One trailing marker per item, on a line of its own at the end of
it:

    //! # Known gaps
    //!
    //! 1. **`$uri->with` replaces a component and cannot remove one**, so there is
    //!    no spelling for "this URI without its fragment". The fix is an options bag
    //!    that can tell an omitted option from a written `null`.
    //!    — owner: unowned

The block itself is written two ways and both are read: a `# Known gaps` heading, and a
`**Known gaps**` bold run with the same items under it. A bold run carries no heading level, so what
ends one is the next bold run or heading in the same doc comment rather than a level comparison.
Eight modules record their gaps that way, and while this file saw only headings the roster could go
green over a module that named nobody for anything it owed.

The index is then *derived* -- this file walks the crates and prints the roster -- rather than a
second copy maintained beside the first, which is the failure `carried-gaps.md` was created to fix
one level up, and repeating it here would be the same mistake with more files. `holes.py` follows
the same rule for a refusal site, and `gaps.py` for a missing conformance case.

**Three owner kinds and no fourth:**

*   **A goal slug** -- `xml-tree`, `unowned-sweep` -- naming an entry in `docs/agent/goals/`. That
    entry closes the gap, or the tag is wrong. A slug rather than a chain number, because a number
    is a *position* and moves the moment anything is inserted ahead of it, which would silently
    re-point every tag in the tree.

*   **A milestone tag** -- `M9`, `M12` -- for a gap a milestone's own plan already covers. This is
    the kind that stops the roster being alarming: scheduled work is not an unclosed hole, and
    conflating the two is what made a hundred-odd careful sentences read as a hundred-odd problems.
    The milestone must be in the plan's table, must not be `done` there, and must be **ahead of the
    program**: everything before `M9` is complete at the end of it, so a tag naming one is not a
    deferral but owed work whose carrier has already been and gone. Such a tag is a section of its
    own and a `past-milestone:` count, refused by `--past-is-an-error`; goal `gap-register`
    § *Standing decisions* is why it is reported here rather than fatal.

*   **`unowned`**, which requires a bullet naming the module's path in one of the two files that
    hold a reason: `carried-gaps.md` § *Unowned*, or `carried-refusals.md` for a gap whose sites
    `holes.py` already carries. That bullet carries the reason, and the reason names what has to be
    *decided* -- "nobody has got to it" is not one. Unowned is a legitimate state and a scheduling
    question for the user; it is never the absence of an answer.

**The gate has no allowlist, and it arrives in pieces.** A tag that resolves to nothing -- naming no
goal on the chain, a milestone that is absent or `done`, or a word that is none of the three kinds
-- always fails `--check`. The rest are flags because the gate is built before the pass it gates,
and a gate that goes red on work nobody has done yet earns exactly one thing: the exemption list
this refuses to have. `--untagged-is-an-error` adds the items that name nobody, `--reasons` adds the
`unowned` ones with no bullet behind them, and `--past-is-an-error` adds the ones deferred to a
milestone already behind the program; the form `verify.py` runs is whichever of them the tree can
currently hold, and every count the flags do not refuse is still printed as a `label: N` line. A gap that cannot be tagged is a gap whose owner has to be
decided, and that decision is cheap exactly once -- when the gap is written.

**A retired owner is reported and does not fail.** A goal that went green without closing the gap it
claimed is a real finding, and it is the same finding `python tools/playbook.py --check` prints for
a `carried-gaps.md` § *Owned* row whose owner is retired -- one behaviour for one fact, in both
files. Striking the owner is a judgement (is the gap closed, or was it left behind?), so the tool
surfaces it and a session decides.

**A heading is not a register.** Owed work stated under a section of its own -- `# What is here, and
what is not yet`, `# What a caller owes` -- is in the one shape nothing can read: no item number, no
owner tag, no way to ask who closes it. The roster names every such heading and counts them on the
`sections outside Known gaps` line, and never fails on one, because the answer is always the same
edit: move what is owed into that file's `# Known gaps` block as numbered items with owners, and
rewrite the heading to say what the module does. The wording is matched on whole words -- `lowers`
and `borrowed` both contain `owe`, and a warning listing those is one nobody reads twice.

**Six registers, and this reads all of them.** A module doc is where a gap belongs, and it is not the
only place this repository writes owed work down: a refusal site's reason is in
`docs/agent/carried-refusals.md`, an unregistered spec member is a key in one of
`crates/nvs-stdlib/tests/`'s four `*-outstanding.txt` ratchets, a guard test named before it was
written is a bullet in `docs/agent/guard-name-debt.md`, a trap that retires when the tree reaches a
state is a playbook bullet's `[until:]` trailer, and `docs/agent/carried-gaps.md` indexes what a
shipped feature still owes. Each answers a different question, so none of them is redundant -- but
until `--registers` there was nowhere to ask *is anything open?* and get one answer, and six answers
is the same as none.

**What `--registers` takes from the other five is the count, and nothing else.** Each already has a
gate over its own discipline: `python tools/playbook.py --check` reads an `[until:]` trailer and a
`carried-gaps.md` § *Owned* row whose owner went green, and
`every_outstanding_key_names_an_owner` in `crates/nvs-stdlib/tests/spec_registry_coverage.rs` reads a
ratchet's `#` owner column. A second opinion here would be the duplicate index the derivation above
exists to avoid, so what a register *holds* is read out of it and what a register *owes* is left to
the gate that owns it. What counts as an entry is asked of `tools/playbook.py` for the same reason.

Usage:

    python tools/owners.py              the roster: untagged, by goal, by milestone, unowned
    python tools/owners.py --registers  one line per register, and how many items each holds
    python tools/owners.py --deferrals  each milestone tag against the scope its plan file states
    python tools/owners.py --untagged   only the items that name nobody
    python tools/owners.py --unowned    only the scheduling questions
    python tools/owners.py --check      exit 1 with a line per tag that resolves to nothing
    python tools/owners.py --check --untagged-is-an-error --reasons
                                        the gate the tree holds today: nothing untagged, every
                                        reason written, a past milestone counted and not refused
    python tools/owners.py --check --past-is-an-error
                                        adds: no gap is deferred to a milestone already passed
    python tools/owners.py --json       the same, as one object
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import goals as goalsmod  # noqa: E402  -- the chain's one reader
import orient as orientmod  # noqa: E402  -- section slicing lives there and is not reimplemented
import playbook as playbookmod  # noqa: E402  -- what an entry is, and the `[until:]` trailer

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "implementation-plan.md"
CARRIED_GAPS = ROOT / "docs" / "agent" / "carried-gaps.md"
CARRIED_REFUSALS = ROOT / "docs" / "agent" / "carried-refusals.md"
GUARD_DEBT = ROOT / "docs" / "agent" / "guard-name-debt.md"
PLAYBOOK = ROOT / "docs" / "agent" / "playbook.md"

#: The ratchets, as a glob: the set is whatever `crates/nvs-stdlib/tests/` holds, so a fifth one
#: joins the register by being written rather than by being listed here.
RATCHETS = "crates/nvs-stdlib/tests/*-outstanding.txt"

#: A ratchet line: the key, then the owner its `#` column names. The column's grammar is
#: `every_outstanding_key_names_an_owner`'s, which is why nothing here resolves what it reads.
RATCHET_KEY = re.compile(r"^([^#]+?)\s*(?:#\s*(\S+))?\s*$")

#: Where a gap may be recorded. A crate's own source and nothing else: a gap in a tool or a doc has
#: no module doc to live in, and `carried-gaps.md` is where those go.
SOURCES = ["crates/*/src/**/*.rs"]

#: A module doc line. The run of them is the doc; a gap block is a heading inside one.
DOC = re.compile(r"^\s*//!(?: ?(.*))?$")

#: A heading inside a doc run, and the bold run that is the block's other opening. Both `# Known
#: gaps` over a numbered list and `# Known gap: <what it is>` over a paragraph are in the tree, and
#: the second is not a lesser kind -- it is one gap stated as prose, so it is one item. `GAPS` reads
#: a heading's title and a bold run's phrase alike, which is what makes the two spellings one block
#: kind; `BOLD` also matches a bold run that is not a gap block, because that is what ends one.
HEADING = re.compile(r"^(#{1,6})\s+(\S.*?)\s*$")
BOLD = re.compile(r"^\*\*(.+?)(?:\*\*|$)")
GAPS = re.compile(r"^Known gaps?\b", re.IGNORECASE)

#: Owed-work wording in a heading that opens no gap block. A section titled `# What is here, and what
#: is not yet` records what its module owes in the one place no roster reads, so the roster counts
#: them and says where, as a warning rather than a gate. The word `gap` under any heading but the
#: block's own is here too: the block reader recognizes `Known gap` and nothing else, so a section
#: titled for a gap is either owed work in the wrong place or a heading using the word for something
#: that is not one, and the edit differs but the reading does not. The alternation is word-bounded
#: because the wording is ordinary English inside longer words -- `lowers` and `borrowed` both
#: contain `owe`, and a warning that names those is read as noise and then not read at all.
OWED = re.compile(r"\b(?:not(?:\s+\w+){0,3}\s+yet|owe[sd]?|owing|still missing|not armed|gaps?)\b",
                  re.IGNORECASE)

#: An item inside a block, and the marker that names its owner. Both list markers count: the crates
#: write a gap list either way, and a `-` block whose five bullets carried one tag between them
#: would hide four gaps behind the fifth. The marker is an em dash because that is what the prose
#: around it uses; it sits on its own line so it can be grepped for and so adding one never reflows
#: a sentence.
ITEM = re.compile(r"^(?:(\d+)\.|[-*])\s+(\S.*)$")
TAG = re.compile(r"^—\s*owner:\s*(\S+)\s*$")
TAG_LIKE = re.compile(r"owner:\s*\S", re.IGNORECASE)

#: The three owner kinds, in the order they are tried. `unowned` also matches the goal grammar, so
#: it is recognised first.
UNOWNED = "unowned"
MILESTONE = re.compile(r"^M(\d+)[A-Z]?$")
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

#: The first milestone a gap may be deferred to. Everything before it is complete at the end of the
#: program, so a tag naming one is work with nothing left to carry it; goal `gap-register`
#: § *Standing decisions* is where that rule is written down and why it is reported here rather
#: than refused. A suffixed milestone -- `M4S`, `M4B` -- is its number's, which is what the suffix
#: means in the plan's table.
FIRST_FUTURE_MILESTONE = 9

#: The plan's milestone table: the *Carried by* cell, then the milestone the row is about. A cell
#: reading `done` is the only thing that means finished -- the plan's own § below the table says so.
PLAN_ROW = re.compile(r"^\|\s*([^|]*?)\s*\|\s*\[(M\d+[A-Z]?)\]")

#: The first bold phrase of an item, which is how every block in the tree opens one.
LEAD = re.compile(r"\*\*(.+?)\*\*", re.S)

#: What `--deferrals` reads an item for: a token in backticks, or a word long enough to be about
#: this gap rather than about English. A plain word matches on its first `STEM` letters, so
#: `inlines` finds a plan that says `inlining`; a backticked token matches whole, because it is
#: already a name. Anything shorter than `STEM` is vocabulary both documents share whatever they
#: are about, and matching on it would make every deferral pass.
KEYWORD = re.compile(r"`([^`\s]{3,})`")
WORD = re.compile(r"\b([A-Za-z][A-Za-z0-9_]{5,})\b")
STEM = 6

#: Words long enough to pass `WORD` that say nothing about a subsystem: this repository's prose
#: vocabulary, which two documents share however unrelated their subjects are. Matching on one
#: would let a plan file scope an item by being written in English.
PROSE = {"because", "instead", "rather", "already", "against", "nothing", "anything", "everything",
         "itself", "second", "single", "whole", "written", "writes", "reading", "carries",
         "answers", "entries", "program", "milestone", "repository", "whatever", "however",
         "therefore", "without", "within", "through", "themselves", "something"}


def rel(path: Path) -> str:
    return path.resolve().relative_to(ROOT).as_posix()


def doc_runs(lines: list[str]):
    """Every contiguous `//!` run, as (first line number, [(line number, text)])."""
    run: list[tuple[int, str]] = []
    for n, line in enumerate(lines, 1):
        m = DOC.match(line)
        if m:
            run.append((n, m.group(1) or ""))
        elif run:
            yield run
            run = []
    if run:
        yield run


def blocks(run: list[tuple[int, str]]):
    """Every gap block in one doc run, with the body under it, in the order they open.

    A heading's body ends at the next heading of the same or a higher level, which is what makes a
    `## Known gaps` nested under a `# ` section stop at its sibling rather than swallowing the rest
    of the doc. A bold run has no level to compare, so its body ends at the next line that opens a
    bold run of its own or at the next heading -- and it *starts* on the label's own line, since the
    rest of that sentence is the block's first line of prose: a run opening `**Known gaps**, beyond
    the ones this crate's docs name: a `switch` case that ...` states its one gap right there.
    """
    heads = [(i, HEADING.match(text)) for i, (_, text) in enumerate(run)]
    heads = [(i, len(m.group(1)), m.group(2)) for i, m in heads if m]
    bolds = [(i, BOLD.match(text)) for i, (_, text) in enumerate(run)]
    bolds = [(i, m.group(1), m.end()) for i, m in bolds if m]
    stops = sorted([i for i, _, _ in heads] + [i for i, _, _ in bolds])
    found = []
    for n, (idx, level, title) in enumerate(heads):
        if not GAPS.match(title):
            continue
        end = len(run)
        for later, later_level, _ in heads[n + 1:]:
            if later_level <= level:
                end = later
                break
        found.append((idx, title, run[idx + 1:end]))
    for idx, phrase, cut in bolds:
        if not GAPS.match(phrase):
            continue
        end = next((i for i in stops if i > idx), len(run))
        line, text = run[idx]
        found.append((idx, phrase, [(line, text[cut:].strip(" ,.:—-"))] + run[idx + 1:end]))
    for idx, title, body in sorted(found, key=lambda block: block[0]):
        yield run[idx][0], title, body


def items_of(body: list[tuple[int, str]], title: str) -> list[dict]:
    """The enumerated items in one block, or the block itself when it enumerates nothing.

    `# Known gap: none of these has a [server] key yet` is a whole block that is one gap, written as
    a paragraph under a heading that states it. Counting it as an item is not a courtesy: an
    unenumerated block that owed no tag would be the allowlist this gate refuses to have, and the
    cheapest one to write.
    """
    starts = [(i, ITEM.match(text)) for i, (_, text) in enumerate(body)]
    starts = [(i, m.group(1)) for i, m in starts if m]
    starts = [(i, number or str(n + 1)) for n, (i, number) in enumerate(starts)]
    if not starts:
        text = "\n".join(t for _, t in body).strip()
        if not text:
            return []
        _, sep, rest = title.partition(":")
        lead = rest.strip() if sep else ""
        if not lead:
            m = LEAD.search(text)
            lead = m.group(1) if m else text
        return [{"num": 1, "line": body[0][0], "lead": one_line(lead), "body": body}]
    out = []
    for n, (idx, number) in enumerate(starts):
        end = starts[n + 1][0] if n + 1 < len(starts) else len(body)
        chunk = body[idx:end]
        text = "\n".join([ITEM.match(chunk[0][1]).group(2)] + [t for _, t in chunk[1:]])
        m = LEAD.search(text)
        out.append({"num": int(number), "line": chunk[0][0],
                    "lead": one_line(m.group(1) if m else text), "body": chunk})
    return out


def one_line(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def tag_of(item: dict) -> tuple[str, str]:
    """`(owner, "")`, or `("", why it could not be read)`."""
    lines = [t.strip() for _, t in item["body"] if t.strip()]
    if not lines:
        return "", "the item is empty"
    tagged = [line for line in lines if TAG.match(line)]
    if len(tagged) > 1:
        return "", "two owner tags in one item"
    if tagged:
        if tagged[0] != lines[-1]:
            return "", "the owner tag is not the item's last line"
        return TAG.match(lines[-1]).group(1), ""
    near = [line for line in lines if TAG_LIKE.search(line)]
    if near:
        return "", "an owner is named but not as a trailing `— owner: <who>` line"
    return "", "no owner tag"


def milestones() -> dict[str, str]:
    """Every milestone in the plan's table, mapped to its *Carried by* cell."""
    if not PLAN.is_file():
        return {}
    found = {}
    for line in PLAN.read_text(encoding="utf-8").split("\n"):
        m = PLAN_ROW.match(line)
        if m:
            found[m.group(2)] = m.group(1).strip()
    return found


def is_past(tag: str) -> bool:
    """Whether a milestone tag names a milestone the program has already walked past."""
    m = MILESTONE.match(tag)
    return bool(m) and int(m.group(1)) < FIRST_FUTURE_MILESTONE


def unowned_paths() -> set[str]:
    """The module paths a reason names, in either of the two files that hold one.

    A bullet is prose and stays prose; what makes it machine-readable is that it already names the
    file whose module doc owns the detail, because the contract's fourth rule asks it to point
    rather than restate. So the link is the path, and no key has to be invented for either side.

    `carried-gaps.md` § *Unowned* is one such file and `carried-refusals.md` is the other, whole:
    a refusal site's reason is written there in full, and the goal's § *Standing decisions* is why
    copying it under § *Unowned* to satisfy this reader would be the second index the gate exists
    to end.
    """
    found: set[str] = set()
    for path, heading in ((CARRIED_GAPS, "Unowned"), (CARRIED_REFUSALS, None)):
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        if heading is not None:
            text = orientmod.slice_section(text, heading) or ""
        found |= set(re.findall(r"crates/[A-Za-z0-9_\-./]+\.rs", text))
    return found


def collect() -> list[dict]:
    """Every gap item in the crates, tagged or not, in file order."""
    paths = sorted({p for pattern in SOURCES for p in ROOT.glob(pattern)})
    found = []
    for path in paths:
        lines = path.read_text(encoding="utf-8").split("\n")
        for run in doc_runs(lines):
            for line, title, body in blocks(run):
                for item in items_of(body, title):
                    owner, why = tag_of(item)
                    found.append({"file": rel(path), "block": line, "heading": title,
                                  "item": item["num"], "line": item["line"], "lead": item["lead"],
                                  "text": one_line(" ".join(t for _, t in item["body"])),
                                  "owner": owner, "why": why})
    return found


def outside_blocks() -> list[dict]:
    """Every module doc heading that words owed work while opening no gap block.

    A heading is what this reads, and prose is not: "the parser does not honour this yet" inside a
    paragraph is a sentence about the code, while a *section* titled for what is missing is a
    register of owed work that nothing counts and no owner tag reaches. The gap block's own heading
    is exempt, since that is the one place the wording belongs.
    """
    found = []
    for path in sorted({p for pattern in SOURCES for p in ROOT.glob(pattern)}):
        for run in doc_runs(path.read_text(encoding="utf-8").split("\n")):
            for line, text in run:
                m = HEADING.match(text)
                if not m or GAPS.match(m.group(2)) or not OWED.search(m.group(2)):
                    continue
                found.append({"file": rel(path), "line": line, "lead": one_line(m.group(2))})
    return found


def entries(path: Path) -> list[dict]:
    """The blocks one of the append-mostly files counts as entries, as `{file, line, lead}`.

    Which block is an entry is `playbook.DECLARING`'s predicate and not a second copy of it: the
    file's shape is that module's fact, and a register disagreeing with the checker about what it
    holds would be exactly the divergence one roster exists to end.
    """
    if not path.is_file():
        return []
    must = playbookmod.DECLARING[path]
    text = path.read_text(encoding="utf-8")
    return [{"file": rel(path), "line": b["start"] + 1, "lead": one_line(b["lead"])}
            for b in playbookmod.blocks(text) if must(b["section"], b["first"])]


def ratchet_keys() -> list[dict]:
    """Every outstanding key in the ratchets, with the owner its `#` column names."""
    found = []
    for path in sorted(ROOT.glob(RATCHETS)):
        for n, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
            text = line.strip()
            if not text or text.startswith("#"):
                continue
            m = RATCHET_KEY.match(text)
            found.append({"file": rel(path), "line": n, "lead": one_line(m.group(1)),
                          "owner": m.group(2) or ""})
    return found


def until_bullets() -> list[dict]:
    """Every playbook bullet whose trailer names a state of the tree that retires it.

    A `reviewed` trailer declares no condition -- `tools/playbook.py`'s module doc is its home and
    says so -- so it is a bullet nothing is owed for, and counting it here would make this register
    a count of the playbook rather than of what the tree still owes.
    """
    if not PLAYBOOK.is_file():
        return []
    found = []
    for b in playbookmod.blocks(PLAYBOOK.read_text(encoding="utf-8")):
        decl = playbookmod.declaration(b["body"])
        if decl and decl[0] != "reviewed":
            found.append({"file": rel(PLAYBOOK), "line": b["start"] + 1,
                          "lead": one_line(b["lead"]), "owner": ""})
    return found


def carried_gaps() -> list[dict]:
    """The index's two halves: § *Owned*'s table rows and § *Unowned*'s bullets."""
    if not CARRIED_GAPS.is_file():
        return []
    text = CARRIED_GAPS.read_text(encoding="utf-8")
    rows = [{"file": rel(CARRIED_GAPS), "line": line + 1, "lead": one_line(gap),
             "owner": owner.strip().strip("`")}
            for line, gap, owner in playbookmod.owned_rows(text)]
    return rows + entries(CARRIED_GAPS)


def registers(found: list[dict]) -> list[dict]:
    """Every place this repository writes owed work down, with what each one currently holds.

    In the order a gap is most often written: the module doc that owes it, then the four files that
    hold what a module doc cannot. `items` is what the register holds now, so a caller that wants
    the roster and a caller that wants one count read the same walk.
    """
    return [
        {"name": "module docs", "where": ", ".join(SOURCES), "items": found,
         "what": "a numbered item under a `# Known gaps` block, tagged with its owner"},
        {"name": "carried-refusals.md", "where": rel(CARRIED_REFUSALS),
         "items": entries(CARRIED_REFUSALS),
         "what": "a run of `nvs-ir` refusal sites an earlier milestone left, numbered from 900"},
        {"name": "outstanding keys", "where": RATCHETS, "items": ratchet_keys(),
         "what": "a spec or migration member `registry::CLASSES` does not declare yet, each key "
                 "naming its owner in a `#` column"},
        {"name": "guard-name-debt.md", "where": rel(GUARD_DEBT), "items": entries(GUARD_DEBT),
         "what": "a guard test `loop-goal.toml` names and the tree does not hold yet"},
        {"name": "playbook until", "where": rel(PLAYBOOK), "items": until_bullets(),
         "what": "a trap whose `[until:]` trailer names the state of the tree that retires it"},
        {"name": "carried-gaps.md", "where": rel(CARRIED_GAPS), "items": carried_gaps(),
         "what": "what a shipped feature still owes, indexed § *Owned* and § *Unowned*"},
    ]


def report_registers(regs: list[dict]) -> None:
    width = max(len(reg["name"]) for reg in regs)
    print("== EVERY PLACE THIS REPOSITORY WRITES OWED WORK DOWN")
    for reg in regs:
        print(f"  {reg['name']:<{width}}  {len(reg['items']):>4} open item(s)  {reg['where']}")
        print(f"  {'':<{width}}       {reg['what']}")
    total = sum(len(reg["items"]) for reg in regs)
    print(f"\n== {total} item(s) open across {len(regs)} register(s)")


def register_line(regs: list[dict]) -> str:
    """The counts on one line, for a mode whose output is a verdict rather than a roster."""
    counts = ", ".join(f"{reg['name']} {len(reg['items'])}" for reg in regs)
    return f"  registers: {counts} -- {len(regs)} register(s)"


def classify(found: list[dict]) -> dict:
    """Sort every item into its kind, and say what is wrong with the ones that are.

    The kinds are the output. `broken` is what `--check` refuses on its own; `untagged`,
    `unreasoned` and `past` are what its three flags add, and `retired` is a finding rather than a
    failure, per this module's own doc.
    """
    chain = {g.slug: g for g in goalsmod.load()}
    plan = milestones()
    paths = unowned_paths()
    out = {"goal": [], "milestone": [], "past": [], "unowned": [], "unreasoned": [],
           "untagged": [], "broken": [], "retired": []}
    for gap in found:
        owner = gap["owner"]
        if not owner:
            out["untagged"].append(gap)
        elif owner == UNOWNED:
            if gap["file"] in paths:
                out["unowned"].append(gap)
            else:
                out["unreasoned"].append({**gap, "why": "`unowned` with no reason naming this file, "
                                                        "in carried-gaps.md § *Unowned* or in "
                                                        "carried-refusals.md"})
        elif MILESTONE.match(owner):
            carried = plan.get(owner)
            if carried is None:
                out["broken"].append({**gap, "why": f"no milestone {owner} in the plan's table"})
            elif is_past(owner):
                out["past"].append({**gap, "why": f"{owner} is behind the program; only M"
                                                  f"{FIRST_FUTURE_MILESTONE} and later is a "
                                                  f"deferral, so this is owed by a goal or nobody"})
            elif carried.lower().startswith("done"):
                out["broken"].append({**gap, "why": f"milestone {owner} is done; a gap it did not "
                                                    f"close is owned by a goal or by nobody"})
            else:
                out["milestone"].append(gap)
        elif SLUG.match(owner):
            goal = chain.get(owner)
            if goal is None:
                out["broken"].append({**gap, "why": f"no goal `{owner}` in docs/agent/goals/"})
            elif goal.retired:
                out["retired"].append(gap)
            else:
                out["goal"].append(gap)
        else:
            out["broken"].append({**gap, "why": f"{owner!r} is neither a goal slug, a milestone "
                                                f"tag nor `unowned`"})
    return out


def plan_file(tag: str) -> Path:
    """The milestone's own file, which is the only place `--deferrals` reads its scope from.

    Goal `gap-register` § *Standing decisions*: a scope sentence written anywhere else does not
    count, and this never edits a plan file to make a tag pass.
    """
    return ROOT / "docs" / "plan" / f"{tag.lower()}.md"


def covers(text: str, gap: dict) -> str:
    """What the milestone's plan says that covers this item, or "" when it says nothing.

    Three ways, strongest first, and each is the plan naming something the item names. The
    **path**: the file the gap is written in, its crate, or the subsystem that crate is --
    `nvs-fmt`'s gaps are scoped by a plan that talks about `nvs fmt`, which is what the tool is
    called in prose. Then a **name the item puts in backticks**, which is the evidence a reader
    would look for. Then, last, a **word of the item's own text**, per `WORD` and `PROSE` above.
    """
    lowered = text.lower()
    body = gap.get("text") or gap["lead"]
    crate = gap["file"].split("/")[1] if "/" in gap["file"] else ""
    for path in (gap["file"], crate, crate.removeprefix("nvs-")):
        if path and path.lower() in lowered:
            return path
    for token in KEYWORD.findall(body):
        if token.lower() in lowered:
            return f"`{token}`"
    for word in WORD.findall(body):
        if word.lower() not in PROSE and word[:STEM].lower() in lowered:
            return word
    return ""


def run_deferrals(kinds: dict) -> int:
    """Every M9-and-later tag, against the milestone file that has to have scoped it."""
    missed = []
    print("== EVERY GAP DEFERRED TO A MILESTONE AHEAD OF THE PROGRAM")
    for owner, gaps in by_owner(kinds["milestone"]).items():
        path = plan_file(owner)
        text = path.read_text(encoding="utf-8") if path.is_file() else ""
        print(f"  {owner} -- {len(gaps)} item(s), against {rel(path)}")
        for gap in gaps:
            print(line_of(gap))
            how = covers(text, gap) if text else ""
            if how:
                print(f"      scoped there by {how}")
            else:
                missed.append((gap, path))
                print(f"      {rel(path)} states no scope covering this item"
                      if text else f"      {rel(path)} does not exist")
    if not kinds["milestone"]:
        print("  none")

    if missed:
        print(f"\n== {len(missed)} deferral(s) name a milestone whose plan does not state the "
              f"scope. Either the item is owned by a goal on the chain, or the milestone's own "
              f"file is where the scope belongs -- written there for its own sake, never to make "
              f"a tag pass.")
        return 1
    print("\n== every deferral names a future milestone whose plan states the scope")
    return 0


def line_of(gap: dict) -> str:
    return f"  {gap['file']}:{gap['line']}  gap {gap['item']}  {gap['lead'][:78]}"


def by_owner(gaps: list[dict]) -> dict[str, list[dict]]:
    grouped: dict[str, list[dict]] = {}
    for gap in gaps:
        grouped.setdefault(gap["owner"], []).append(gap)
    return dict(sorted(grouped.items()))


#: Every kind the summary counts, and the label it is counted under. One line each and one count
#: per line: an acceptance `want` is a substring match, so `0 untagged` would be satisfied by
#: `10 untagged` and a run that got worse would read as the run that was asked for.
LABELS = [("goal", "goal-owned"), ("milestone", "milestone-owned"), ("past", "past-milestone"),
          ("unowned", "unowned"), ("untagged", "untagged"), ("broken", "broken-tag"),
          ("unreasoned", "unreasoned"), ("retired", "retired-owner")]


def count_lines(kinds: dict) -> list[str]:
    return [f"  {label}: {len(kinds[kind])}" for kind, label in LABELS]


def report(kinds: dict, found: list[dict], regs: list[dict], sections: list[dict]) -> None:
    files = len({gap["file"] for gap in found})
    blocks_seen = len({(gap["file"], gap["block"]) for gap in found})

    print("== ITEMS THAT NAME NOBODY")
    for gap in kinds["untagged"]:
        print(line_of(gap))
        print(f"      {gap['why']}")
    if not kinds["untagged"]:
        print("  none -- every recorded gap names a goal, a milestone or nobody on purpose")

    if kinds["broken"]:
        print("\n== TAGS THAT DO NOT RESOLVE")
        for gap in kinds["broken"]:
            print(line_of(gap))
            print(f"      {gap['why']}")

    if kinds["unreasoned"]:
        print("\n== `unowned` WITH NO REASON WRITTEN DOWN")
        for gap in kinds["unreasoned"]:
            print(line_of(gap))
            print(f"      {gap['why']}")

    if kinds["past"]:
        print("\n== DEFERRED TO A MILESTONE THE PROGRAM HAS ALREADY PASSED")
        for gap in kinds["past"]:
            print(line_of(gap))
            print(f"      {gap['why']}")

    if kinds["retired"]:
        print("\n== OWNERS THAT WENT GREEN WITHOUT CLOSING THE GAP")
        for gap in kinds["retired"]:
            print(line_of(gap))
            print(f"      goal `{gap['owner']}` is retired; close the gap or re-owner it")

    print("\n== OWNED BY A GOAL ON THE CHAIN")
    for owner, gaps in by_owner(kinds["goal"]).items():
        print(f"  goal `{owner}` -- {len(gaps)} item(s)")
        for gap in gaps:
            print(line_of(gap))
    if not kinds["goal"]:
        print("  none")

    print("\n== SCHEDULED BY A MILESTONE'S PLAN")
    for owner, gaps in by_owner(kinds["milestone"]).items():
        print(f"  {owner} -- {len(gaps)} item(s)")
        for gap in gaps:
            print(line_of(gap))
    if not kinds["milestone"]:
        print("  none")

    print("\n== UNOWNED -- the scheduling questions, each with its reason in carried-gaps.md "
          "or carried-refusals.md")
    for gap in kinds["unowned"]:
        print(line_of(gap))
    if not kinds["unowned"]:
        print("  none")

    if sections:
        print("\n== OWED WORK UNDER A HEADING OF ITS OWN, WHERE NO OWNER TAG REACHES IT")
        for sec in sections:
            print(f"  {sec['file']}:{sec['line']}  {sec['lead'][:78]}")
        print("  move each one into that file's `# Known gaps` block as a numbered item naming its "
              "owner, and rewrite the heading to say what the module does")

    print(f"\n== {len(found)} item(s) in {blocks_seen} block(s) across {files} file(s)")
    for line in count_lines(kinds):
        print(line)
    print(f"  sections outside Known gaps: {len(sections)}")
    print(f"  of the {len(found)}, {len(kinds['retired'])} owned by a retired goal -- a finding "
          f"rather than a failure, and a session decides it")

    print()
    report_registers(regs)


def run_check(kinds: dict, regs: list[dict], untagged_is_an_error: bool, reasons: bool,
              past_is_an_error: bool) -> int:
    """One line per item the gate refuses, and 1 if there were any.

    What is fatal grows with the flags, and this module's doc says why the gate arrives in pieces.
    Whatever is *not* fatal in this run is still counted below, so a check that passes never reads
    as an inventory that is clean.
    """
    bad = list(kinds["broken"])
    if untagged_is_an_error:
        bad += kinds["untagged"]
    if reasons:
        bad += kinds["unreasoned"]
    if past_is_an_error:
        bad += kinds["past"]
    for gap in sorted(bad, key=lambda g: (g["file"], g["line"])):
        print(f"{gap['file']}:{gap['line']}: gap {gap['item']} -- {gap['why']}")
    if bad:
        print(f"owners.py: {len(bad)} recorded gap(s) name an owner this run refuses, each for the "
              f"reason on its own line. An owner is a live goal on the chain, a milestone at "
              f"M{FIRST_FUTURE_MILESTONE} or later whose plan covers the item, or `unowned` with a "
              f"reason bullet behind it, written as the item's last line "
              f"`— owner: <who>`; `python tools/owners.py --help` is what each kind asserts.")
        for line in count_lines(kinds):
            print(line)
        print(register_line(regs))
        return 1
    named = sum(len(kinds[k])
                for k in ("goal", "milestone", "past", "unowned", "unreasoned", "retired"))
    print(f"owners.py: every one of the {named} tagged gap(s) names an owner the chain or the plan "
          f"knows")
    for kind, flag, what in (("untagged", "untagged-is-an-error", "name nobody"),
                             ("unreasoned", "reasons",
                              "are `unowned` with no reason written down"),
                             ("past", "past-is-an-error",
                              "are deferred to a milestone the program has passed")):
        if kinds[kind]:
            print(f"  {len(kinds[kind])} recorded gap(s) {what} -- not refused in this run; "
                  f"`--{flag}` refuses them")
    for line in count_lines(kinds):
        print(line)
    print(register_line(regs))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--registers", action="store_true",
                    help="one line per register a gap is written in, and what each holds")
    ap.add_argument("--deferrals", action="store_true",
                    help="every milestone tag, against the scope its own plan file states")
    ap.add_argument("--untagged", action="store_true", help="only the items that name nobody")
    ap.add_argument("--unowned", action="store_true", help="only the scheduling questions")
    ap.add_argument("--check", action="store_true", help="exit 1 on a tag that resolves to nothing")
    ap.add_argument("--untagged-is-an-error", action="store_true",
                    help="with --check: an item that names nobody fails too")
    ap.add_argument("--reasons", action="store_true",
                    help="with --check: an `unowned` with no carried-gaps.md bullet fails too")
    ap.add_argument("--past-is-an-error", action="store_true",
                    help="with --check: a tag naming a milestone the program has passed fails too")
    ap.add_argument("--json", action="store_true", help="one JSON object instead")
    opts = ap.parse_args()

    for stream in (sys.stdout, sys.stderr):  # a gap's lead is prose, and consoles here are cp1252
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")

    found = collect()
    kinds = classify(found)
    regs = registers(found)
    sections = outside_blocks()

    if opts.json:
        counts = {reg["name"]: len(reg["items"]) for reg in regs}
        print(json.dumps({**kinds, "registers": counts, "sections": sections}, indent=2))
        return 0
    if opts.registers:
        report_registers(regs)
        return 0
    if opts.deferrals:
        return run_deferrals(kinds)
    if opts.check:
        return run_check(kinds, regs, opts.untagged_is_an_error, opts.reasons,
                         opts.past_is_an_error)
    if opts.untagged:
        for gap in kinds["untagged"]:
            print(line_of(gap))
        print(f"\n  {len(kinds['untagged'])} of {len(found)} item(s) name nobody")
        return 0
    if opts.unowned:
        for gap in kinds["unowned"]:
            print(line_of(gap))
        print(f"\n  {len(kinds['unowned'])} item(s), each with its reason in "
              f"docs/agent/carried-gaps.md § *Unowned* or docs/agent/carried-refusals.md")
        return 0
    report(kinds, found, regs, sections)
    return 0


if __name__ == "__main__":
    sys.exit(main())
