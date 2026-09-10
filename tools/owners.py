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
    The milestone must exist and must not be `done` in the plan's table.

*   **`unowned`**, which requires a bullet in `carried-gaps.md` § *Unowned* naming the module's
    path. That bullet carries the reason, and the reason names what has to be *decided* -- "nobody
    has got to it" is not one. Unowned is a legitimate state and a scheduling question for the
    user; it is never the absence of an answer.

**The gate has no allowlist, and it arrives in three pieces.** A tag that resolves to nothing --
naming no goal on the chain, a milestone that is absent or `done`, or a word that is none of the
three kinds -- always fails `--check`. The other two halves are flags because the gate is built
before the pass it gates, and a gate that goes red on work nobody has done yet earns exactly one
thing: the exemption list this refuses to have. `--untagged-is-an-error` adds the items that name
nobody, and `--reasons` adds the `unowned` ones with no bullet behind them; the full form is all
three, and that is what `verify.py` runs. A gap that cannot be tagged is a gap whose owner has to be
decided, and that decision is cheap exactly once -- when the gap is written.

**A retired owner is reported and does not fail.** A goal that went green without closing the gap it
claimed is a real finding, and it is the same finding `python tools/playbook.py --check` prints for
a `carried-gaps.md` § *Owned* row whose owner is retired -- one behaviour for one fact, in both
files. Striking the owner is a judgement (is the gap closed, or was it left behind?), so the tool
surfaces it and a session decides.

Usage:

    python tools/owners.py              the roster: untagged, by goal, by milestone, unowned
    python tools/owners.py --untagged   only the items that name nobody
    python tools/owners.py --unowned    only the scheduling questions
    python tools/owners.py --check      exit 1 with a line per tag that resolves to nothing
    python tools/owners.py --check --untagged-is-an-error --reasons
                                        the whole gate: nothing untagged, every reason written
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

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "implementation-plan.md"
CARRIED_GAPS = ROOT / "docs" / "agent" / "carried-gaps.md"

#: Where a gap may be recorded. A crate's own source and nothing else: a gap in a tool or a doc has
#: no module doc to live in, and `carried-gaps.md` is where those go.
SOURCES = ["crates/*/src/**/*.rs"]

#: A module doc line. The run of them is the doc; a gap block is a heading inside one.
DOC = re.compile(r"^\s*//!(?: ?(.*))?$")

#: A heading inside a doc run, and the two spellings the crates use for the block. Both `# Known
#: gaps` over a numbered list and `# Known gap: <what it is>` over a paragraph are in the tree, and
#: the second is not a lesser kind -- it is one gap stated as prose, so it is one item.
HEADING = re.compile(r"^(#{1,6})\s+(\S.*?)\s*$")
GAPS = re.compile(r"^Known gaps?\b", re.IGNORECASE)

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
MILESTONE = re.compile(r"^M\d+[A-Z]?$")
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

#: The plan's milestone table: the *Carried by* cell, then the milestone the row is about. A cell
#: reading `done` is the only thing that means finished -- the plan's own § below the table says so.
PLAN_ROW = re.compile(r"^\|\s*([^|]*?)\s*\|\s*\[(M\d+[A-Z]?)\]")

#: The first bold phrase of an item, which is how every block in the tree opens one.
LEAD = re.compile(r"\*\*(.+?)\*\*", re.S)


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
    """Every `# Known gaps` heading in one doc run, with the body under it.

    The body ends at the next heading of the same or a higher level, which is what makes a `## Known
    gaps` nested under a `# ` section stop at its sibling rather than swallowing the rest of the doc.
    """
    heads = [(i, HEADING.match(text)) for i, (_, text) in enumerate(run)]
    heads = [(i, len(m.group(1)), m.group(2)) for i, m in heads if m]
    for n, (idx, level, title) in enumerate(heads):
        if not GAPS.match(title):
            continue
        end = len(run)
        for later, later_level, _ in heads[n + 1:]:
            if later_level <= level:
                end = later
                break
        yield run[idx][0], title, run[idx + 1:end]


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
        lead = rest.strip() if sep else (LEAD.search(text).group(1) if LEAD.search(text) else text)
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


def unowned_paths() -> set[str]:
    """The module paths named by a `carried-gaps.md` § *Unowned* bullet.

    A bullet is prose and stays prose; what makes it machine-readable is that it already names the
    file whose module doc owns the detail, because the contract's fourth rule asks it to point
    rather than restate. So the link is the path, and no key has to be invented for either side.
    """
    if not CARRIED_GAPS.is_file():
        return set()
    section = orientmod.slice_section(CARRIED_GAPS.read_text(encoding="utf-8"), "Unowned") or ""
    return set(re.findall(r"crates/[A-Za-z0-9_\-./]+\.rs", section))


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
                                  "owner": owner, "why": why})
    return found


def classify(found: list[dict]) -> dict:
    """Sort every item into its kind, and say what is wrong with the ones that are.

    The kinds are the output. `broken` is what `--check` refuses on its own; `untagged` and
    `unreasoned` are what its two flags add, and `retired` is a finding rather than a failure, per
    this module's own doc.
    """
    chain = {g.slug: g for g in goalsmod.load()}
    plan = milestones()
    paths = unowned_paths()
    out = {"goal": [], "milestone": [], "unowned": [], "unreasoned": [], "untagged": [],
           "broken": [], "retired": []}
    for gap in found:
        owner = gap["owner"]
        if not owner:
            out["untagged"].append(gap)
        elif owner == UNOWNED:
            if gap["file"] in paths:
                out["unowned"].append(gap)
            else:
                out["unreasoned"].append({**gap, "why": "`unowned` with no bullet in "
                                                        "carried-gaps.md § *Unowned* naming this "
                                                        "file"})
        elif MILESTONE.match(owner):
            carried = plan.get(owner)
            if carried is None:
                out["broken"].append({**gap, "why": f"no milestone {owner} in the plan's table"})
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


def line_of(gap: dict) -> str:
    return f"  {gap['file']}:{gap['line']}  gap {gap['item']}  {gap['lead'][:78]}"


def by_owner(gaps: list[dict]) -> dict[str, list[dict]]:
    grouped: dict[str, list[dict]] = {}
    for gap in gaps:
        grouped.setdefault(gap["owner"], []).append(gap)
    return dict(sorted(grouped.items()))


def report(kinds: dict, found: list[dict]) -> None:
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

    print("\n== UNOWNED -- the scheduling questions, each with its reason in carried-gaps.md")
    for gap in kinds["unowned"]:
        print(line_of(gap))
    if not kinds["unowned"]:
        print("  none")

    print(f"\n== {len(found)} item(s) in {blocks_seen} block(s) across {files} file(s)")
    print(f"  {len(kinds['goal'])} owned by a goal, {len(kinds['milestone'])} scheduled by a "
          f"milestone, {len(kinds['unowned'])} unowned")
    print(f"  {len(kinds['untagged'])} untagged, {len(kinds['broken'])} tagged wrongly, "
          f"{len(kinds['unreasoned'])} unowned with no reason, {len(kinds['retired'])} owned by a "
          f"retired goal")


def run_check(kinds: dict, untagged_is_an_error: bool, reasons: bool) -> int:
    """One line per item the gate refuses, and 1 if there were any.

    What is fatal grows with the flags, and this module's doc says why the gate arrives in pieces.
    Whatever is *not* fatal in this run is still counted on the last line, so a check that passes
    never reads as an inventory that is clean.
    """
    bad = list(kinds["broken"])
    if untagged_is_an_error:
        bad += kinds["untagged"]
    if reasons:
        bad += kinds["unreasoned"]
    for gap in sorted(bad, key=lambda g: (g["file"], g["line"])):
        print(f"{gap['file']}:{gap['line']}: gap {gap['item']} -- {gap['why']}")
    if bad:
        print(f"owners.py: {len(bad)} recorded gap(s) name no owner this tool can resolve. Add "
              f"`— owner: <goal slug|milestone|unowned>` as the item's last line; "
              f"`python tools/owners.py --help` is the three kinds and what each one asserts.")
        return 1
    named = sum(len(kinds[k]) for k in ("goal", "milestone", "unowned", "unreasoned", "retired"))
    print(f"owners.py: every one of the {named} tagged gap(s) names an owner the chain or the plan "
          f"knows")
    for kind, what in (("untagged", "name nobody"),
                       ("unreasoned", "are `unowned` with no reason written down")):
        if kinds[kind]:
            print(f"  {len(kinds[kind])} recorded gap(s) {what} -- not refused in this run; "
                  f"`--{'untagged-is-an-error' if kind == 'untagged' else 'reasons'}` refuses them")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--untagged", action="store_true", help="only the items that name nobody")
    ap.add_argument("--unowned", action="store_true", help="only the scheduling questions")
    ap.add_argument("--check", action="store_true", help="exit 1 on a tag that resolves to nothing")
    ap.add_argument("--untagged-is-an-error", action="store_true",
                    help="with --check: an item that names nobody fails too")
    ap.add_argument("--reasons", action="store_true",
                    help="with --check: an `unowned` with no carried-gaps.md bullet fails too")
    ap.add_argument("--json", action="store_true", help="one JSON object instead")
    opts = ap.parse_args()

    for stream in (sys.stdout, sys.stderr):  # a gap's lead is prose, and consoles here are cp1252
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")

    found = collect()
    kinds = classify(found)

    if opts.json:
        print(json.dumps({k: v for k, v in kinds.items()}, indent=2))
        return 0
    if opts.check:
        return run_check(kinds, opts.untagged_is_an_error, opts.reasons)
    if opts.untagged:
        for gap in kinds["untagged"]:
            print(line_of(gap))
        print(f"\n  {len(kinds['untagged'])} of {len(found)} item(s) name nobody")
        return 0
    if opts.unowned:
        for gap in kinds["unowned"]:
            print(line_of(gap))
        print(f"\n  {len(kinds['unowned'])} item(s), each with its reason in "
              f"docs/agent/carried-gaps.md § *Unowned*")
        return 0
    report(kinds, found)
    return 0


if __name__ == "__main__":
    sys.exit(main())
