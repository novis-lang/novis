#!/usr/bin/env python3
"""Every shape the language still refuses, as a worklist a session can take an item off.

M4's frontier is not coverage and not depth: it is the set of shapes that compile in the front end
and then refuse below it -- `nvs-ir` panics naming itself, `nvs-codegen` returns
`CodegenError::Unsupported`, or the checker types an expression nothing lowers. `gaps.py` answers
"which claim is the corpus missing"; this answers "which shape does the language not have yet", and
the two never overlap.

The list is derived, never copied. Three live sources:

*   **The refusal sites themselves**, read out of `crates/nvs-ir/src/` and `crates/nvs-codegen/src/`.
    A site is a `panic!`/`todo!`/`unimplemented!`/`assert!` whose message claims a shape it will not
    take, or a `CodegenError::Unsupported`, where the type is the claim. This is the honest
    inventory: a hole that stops panicking has left it, and one somebody adds appears without anyone
    updating a list. Both halves of the recognizer are load-bearing and the comment on `CONSTRUCT`
    below says why neither alone is.

*   **The goal's item list**, read out of `docs/agent/loop-goal.md`, and **the carried ones**, read  # check-links:retired
    out of `docs/agent/carried-refusals.md` -- the holes an earlier milestone left, which no current
    goal can claim and which a goal switch would otherwise orphan. Every numbered item carries its
    `crates/…/file.rs:NN` anchors, so a site is attributed to the item whose anchors sit in the same
    file. A site no item claims is the interesting output -- it is either a hole nobody scheduled or
    a decision nobody wrote down, and both are worth a session's attention before the code is.

*   **The artefacts each stage owes**, read out of `loop-goal.toml`'s `cases` lists. A named `.nvst`
    case that is not on disk is one item's remaining proof.

Nothing here is a gate. `tools/loop.py` decides whether the goal is met; this only says where the
work is, so that no session spends its context re-deriving it.

    python tools/holes.py                  the summary: sites per item, and what is unattributed
    python tools/holes.py --item 7         one item -- its anchors, its refusal sites, its cases
    python tools/holes.py --unattributed   only the sites no item claims
    python tools/holes.py --cases          only the `.nvst`/differential artefacts still missing
    python tools/holes.py --sites          every site, file by file, with its message
    python tools/holes.py --guarded        every `guarded_by!` site and the code it names
    python tools/holes.py --json           one JSON object instead
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GOAL_MD = ROOT / "docs" / "agent" / "loop-goal.md"
GOAL_TOML = ROOT / "docs" / "agent" / "loop-goal.toml"

# The second source of items, and the only one a goal switch cannot drop. `goal-switch.py` carries
# the outgoing goal's checks forward and its unclosed items not at all, so a hole an earlier
# milestone left arrives claimed by nobody the moment the chain advances -- twice so far, both times
# on the same seventeen sites. That inventory lives here instead of being inherited by hand.
CARRIED_MD = ROOT / "docs" / "agent" / "carried-refusals.md"

# Carried items number from here, in the file itself rather than by an offset applied on read, so
# that `--item N` takes the number the document shows. Enforced below: a goal's own items start at 1
# and never reach this, so the two numberings cannot collide silently.
CARRIED_BASE = 900

# The registry is the one home for what an `E`-code constant spells, so a guarded site names the
# constant and this reads the number off that file rather than carrying a second copy of the map.
REGISTRY = ROOT / "crates" / "nvs-diagnostics" / "src" / "lib.rs"

# Where a refusal can live. Both crates lower; nothing else does.
SOURCES = ["crates/nvs-ir/src", "crates/nvs-codegen/src"]

# A site is read from the CONSTRUCT that carries the message and from the CLAIM the message makes,
# and it takes both.
#
# The construct alone is not the answer, even though it is the one the docstring above describes:
# 89 panic-family sites sit in these two crates and 62 of them are engine invariants -- "`foreach`
# lost the `Env` binding `{name}` it walks" -- which no program reaches and which will still be
# there when the last hole is closed. Counting those would put this tool's own end state, and the
# ratchet in `crates/nvs-ir/tests/refusals.rs` that reads it, permanently out of reach. Separating
# the two by wording is what does not work: an invariant names the front-end guarantee it trusts,
# and so do several real refusals. Making it mechanical needs the *source* to say which kind it is,
# the way `CodegenError` already distinguishes `Internal` from `Unsupported`; nvs-ir has no such
# spelling, and giving it one is a change to that crate rather than to this tool.
#
# The claim alone is not the answer either: an unanchored match reads a doc comment, a diagnostic's
# help text or a test fixture as a site, which is what the comment skip in `literals` was already
# working around one case at a time.
CONSTRUCT = re.compile(
    r"(?:panic|todo|unimplemented)!\s*\(\s*$"
    r"|(?:debug_)?assert(?:_eq|_ne)?!\s*\([^;{}]*$"
)
# What a refusal *claims*, as a shape rather than as a sentence: it names what it does take and
# stops there -- "only lowers X", "lowers X only through Y", "converts ... only" -- or it says
# outright that it has no arm. This replaced a match on three fixed phrasings, which read 4 sites
# where there are 17: `lowers an array-element write only through a bare local` is the same claim
# in the same house style and was invisible to it, and so was every message an `assert!` carries.
REFUSAL = re.compile(
    r"does not (?:yet )?lower"
    r"|no lowering for"
    r"|has no arm for"
    r"|only (?:lowers|converts|stages|emits|takes|accepts|handles)"
    r"|(?:lowers|converts|stages|emits|reaches) [^.;]{0,90}?\bonly\b",
    re.IGNORECASE,
)
# Everything `CodegenError::Unsupported` carries is a refusal whatever it says, so the type is the
# claim and no wording test applies. Anchored to the constructor's own opening paren: an unanchored
# search over the window read the `internal(...)` calls three lines below one of them as sites.
UNSUPPORTED = re.compile(r"CodegenError::Unsupported\s*\(\s*(?:format!\s*\(\s*)?$")
# ...except the ones that are engine bugs wearing the same type. A unit that holds a call and not
# its callee was assembled wrong; that is nobody's language hole.
ENGINE = re.compile(r"this is a bug|declares no (?:descriptor|slot)|which this unit", re.IGNORECASE)
# The other close a refusal site can take, and the reason this tool has two lists rather than one
# number. `lower::guarded_by!` says the shape never arrives -- the front end refuses it where it is
# written -- and names the code that does the refusing, so it is a guarantee to check rather than a
# hole to count: the construct is not a `panic!` and `sites` above therefore does not see it. What
# checks it is `crates/nvs-ir/tests/refusals.rs`, which holds every code listed here to a
# conformance case expecting it, so a guard naming a code nothing raises fails rather than passing
# quietly. A path qualifier is optional because a site may spell the constant imported or through
# `code::`.
GUARDED = re.compile(r"guarded_by!\s*\(\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*([A-Z][A-Z0-9_]*)")
DECLARES_CODE = re.compile(r"pub const ([A-Z][A-Z0-9_]*): Code = Code::new\(\"([A-Z]\d+)\"\)")
ANCHOR = re.compile(r"(crates/[A-Za-z0-9_\-./]+\.rs):(\d+)")
ITEM = re.compile(r"^(\d+)\. \*\*(.+?)\*\*", re.MULTILINE)
# A backticked snake_case word in an item's prose is how it names the function it changes --
# `lower_binary`, `emit_binop`, `landing_block`. That is a far sharper key than a line number,
# which every edit above the site moves.
NAMED_FN = re.compile(r"`([a-z_][a-z0-9_]{4,})`")
DEFINES_FN = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?"
                        r"(?:extern\s+\"[^\"]+\"\s+)?fn\s+([a-z_][a-z0-9_]*)")


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def literals(text: str):
    """Every string literal in `text` that is not inside a comment, as (line, body).

    Rust wraps a long message across lines with a trailing backslash, so a literal is read to its
    closing quote rather than to the end of the line -- reading one line gives half a sentence. The
    comment skip is what keeps a crate's own *Known gaps* prose, which says "does not lower" in
    nearly every entry, out of the inventory of things that actually refuse."""
    i, line, bol = 0, 1, 0
    while i < len(text):
        ch = text[i]
        if ch == "\n":
            line, bol, i = line + 1, i + 1, i + 1
            continue
        if ch == "/" and text[i:i + 2] == "//":
            nl = text.find("\n", i)
            i = len(text) if nl < 0 else nl
            continue
        if ch != '"':
            i += 1
            continue
        start, started = i, line
        out, i = [], i + 1
        while i < len(text):
            ch = text[i]
            if ch == "\\":
                nxt = text[i + 1] if i + 1 < len(text) else ""
                if nxt == "\n":
                    line, i = line + 1, i + 2
                    while i < len(text) and text[i] in " \t":
                        i += 1
                    continue
                out.append(nxt)
                i += 2
                continue
            if ch == "\n":
                line, i = line + 1, i + 1
                out.append(" ")
                continue
            if ch == '"':
                i += 1
                break
            out.append(ch)
            i += 1
        yield started, start, re.sub(r"\s+", " ", "".join(out)).strip()


def sites() -> list[dict]:
    """Every refusal site under `SOURCES`, in file order."""
    found = []
    for source in SOURCES:
        for path in sorted((ROOT / source).rglob("*.rs")):
            # A crate's own tests refuse things on purpose, and no item anchors a test file.
            if path.name == "tests.rs" or path.parent.name == "tests":
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            lines = text.split("\n")
            for line, at, message in literals(text):
                # The window is wide enough to reach back over an `assert!`'s condition and over a
                # `CodegenError::Unsupported(format!(` pair, both of which sit between the
                # construct and the literal that says what it refuses.
                near = text[max(0, at - 240):at]
                unsupported = bool(UNSUPPORTED.search(near))
                if not (unsupported or CONSTRUCT.search(near)):
                    continue
                if not (unsupported or REFUSAL.search(message)):
                    continue
                # `#[error("nvs-codegen does not lower {0} yet")]` is the variant's Display impl,
                # not a site: the sites are the places that construct it, and they are counted.
                if ENGINE.search(message) or near.rstrip().endswith("#[error("):
                    continue
                found.append({
                    "file": rel(path),
                    "line": line,
                    "fn": enclosing_fn(lines, line),
                    "message": message or "(no literal message at the site)",
                })
    return found


def registry() -> dict[str, str]:
    """Every `E`-code constant the registry declares, as `name -> code`."""
    if not REGISTRY.exists():
        return {}
    text = REGISTRY.read_text(encoding="utf-8", errors="replace")
    return {name: code for name, code in DECLARES_CODE.findall(text)}


def guarded() -> list[dict]:
    """Every `guarded_by!` site under `SOURCES`, in file order.

    A site whose constant the registry does not declare comes back with an empty `code`, which is
    what the test reading this reports rather than skipping: a guard naming nothing is the one
    shape that would otherwise look like a closed gap while guaranteeing nothing."""
    declared = registry()
    found = []
    for source in SOURCES:
        for path in sorted((ROOT / source).rglob("*.rs")):
            if path.name == "tests.rs" or path.parent.name == "tests":
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            lines = text.split("\n")
            for match in GUARDED.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                # The macro's own doc comment shows a call, and so does the crate preamble that
                # contrasts the two closes, so a match inside a comment is prose about the
                # spelling rather than a site written in it -- the same reason `literals` skips
                # comments for the other list.
                column = match.start() - (text.rfind("\n", 0, match.start()) + 1)
                remark = lines[line - 1].find("//")
                if remark != -1 and remark < column:
                    continue
                found.append({
                    "file": rel(path),
                    "line": line,
                    "fn": enclosing_fn(lines, line),
                    "const": match.group(1),
                    "code": declared.get(match.group(1), ""),
                })
    return found


def enclosing_fn(lines: list[str], line: int) -> str:
    """The name of the function a site sits in, or "" when it sits at file scope."""
    for n in range(min(line, len(lines)) - 1, -1, -1):
        found = DEFINES_FN.match(lines[n])
        if found:
            return found.group(1)
    return ""


def items() -> list[dict]:
    """The goal's numbered items and the carried ones, with the anchors each names."""
    goal = items_in(GOAL_MD)
    carried = items_in(CARRIED_MD)
    for item in carried:
        if item["n"] < CARRIED_BASE:
            raise SystemExit(
                f"{rel(CARRIED_MD)}: item {item['n']} must be numbered from {CARRIED_BASE} "
                f"so it cannot collide with a goal's own item {item['n']}"
            )
    return goal + carried


def items_in(path: Path) -> list[dict]:
    """One item list, read out of `path`."""
    if not path.exists():
        return []
    text = path.read_text(encoding="utf-8", errors="replace")
    marks = list(ITEM.finditer(text))
    out = []
    for n, match in enumerate(marks):
        end = marks[n + 1].start() if n + 1 < len(marks) else len(text)
        body = text[match.start():end]
        anchors = [(f, int(line)) for f, line in ANCHOR.findall(body)]
        out.append({
            "n": int(match.group(1)),
            "title": re.sub(r"\s+", " ", match.group(2)).strip(),
            "files": sorted({f for f, _ in anchors}),
            "anchors": [f"{f}:{line}" for f, line in anchors],
            "functions": sorted(set(NAMED_FN.findall(body))),
        })
    return out


def named_cases() -> list[dict]:
    """Every `.nvst` artefact `loop-goal.toml` names, and whether it is on disk yet."""
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover -- 3.10 and older
        return []
    if not GOAL_TOML.exists():
        return []
    spec = tomllib.loads(GOAL_TOML.read_text(encoding="utf-8", errors="replace"))
    out = []
    for check in spec.get("check", []):
        for case in check.get("cases", []):
            out.append({
                "suite": check.get("name", "?"),
                "stage": check.get("stage", "?"),
                "path": case,
                "written": (ROOT / case).exists(),
            })
    return out


def attribute(found: list[dict], scheduled: list[dict]) -> dict[int, list[dict]]:
    """Map each site onto the item that claims it.

    By the **enclosing function** first, because that is what an item's prose actually names and it
    survives every edit above the site; by nearest anchor in the same file only as a fallback, and
    such a site is marked `by="file"` so nobody reads a guess as a fact. A site neither claims is
    left unattributed, which is the answer worth printing: it is a hole nobody scheduled or a
    decision nobody wrote down."""
    by_item: dict[int, list[dict]] = {}
    for site in found:
        claimed, how = None, ""
        for item in scheduled:
            if site["fn"] and site["fn"] in item["functions"]:
                claimed, how = item["n"], "fn"
                break
        if claimed is None:
            distance = None
            for item in scheduled:
                for anchor in item["anchors"]:
                    path, _, line = anchor.rpartition(":")
                    if path != site["file"]:
                        continue
                    gap = abs(int(line) - site["line"])
                    if distance is None or gap < distance:
                        claimed, how, distance = item["n"], "file", gap
        by_item.setdefault(claimed if claimed is not None else 0, []).append(dict(site, by=how))
    return by_item


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--item", type=int, help="one item: its anchors, its sites and its cases")
    ap.add_argument("--unattributed", action="store_true", help="only sites no item claims")
    ap.add_argument("--cases", action="store_true", help="only the named cases not yet written")
    ap.add_argument("--sites", action="store_true", help="every site, file by file")
    ap.add_argument("--guarded", action="store_true",
                    help="every `guarded_by!` site and the code it names")
    ap.add_argument("--json", action="store_true", help="one JSON object instead")
    opts = ap.parse_args()

    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")

    found, scheduled, cases = sites(), items(), named_cases()
    guards = guarded()
    by_item = attribute(found, scheduled)
    absent = [c for c in cases if not c["written"]]

    if opts.json:
        print(json.dumps({
            "sites": found,
            "guarded": guards,
            "items": [dict(i, sites=len(by_item.get(i["n"], []))) for i in scheduled],
            "unattributed": by_item.get(0, []),
            "cases": cases,
        }, indent=2))
        return 0

    if opts.guarded:
        print(f"{len(guards)} guarded site(s)\n")
        current = ""
        for site in guards:
            if site["file"] != current:
                current = site["file"]
                print(f"  {current}")
            # One token, so that the test reading this column fails on a guard naming a constant
            # the registry does not declare instead of parsing a sentence as a code.
            code = site["code"] or "no-such-constant"
            print(f"    :{site['line']:<5} {code:<16} {site['const']}  in {site['fn'] or '(file scope)'}")
        if guards:
            print("\nEach says the shape never arrives because the code beside it refuses one where")
            print("it is written. crates/nvs-ir/tests/refusals.rs holds each to a conformance case.")
        return 0

    if opts.item is not None:
        one = next((i for i in scheduled if i["n"] == opts.item), None)
        if one is None:
            print(f"no item {opts.item} in {rel(GOAL_MD)}")
            return 1
        print(f"item {one['n']}: {one['title']}\n")
        print("  anchors")
        for anchor in one["anchors"] or ["(none -- add them to the goal)"]:
            print(f"    {anchor}")
        mine = by_item.get(one["n"], [])
        print(f"\n  refusal sites in those files: {len(mine)}")
        for site in mine:
            print(f"    {site['file']}:{site['line']}  {site['message'][:110]}")
        owed = [c for c in absent if any(word in c["path"] for word in one["title"].lower().split() if len(word) > 5)]
        if owed:
            print("\n  cases that may belong to it (name match, judge it yourself)")
            for case in owed:
                print(f"    {case['path']}")
        return 0

    if opts.cases:
        print(f"{len(absent)} of {len(cases)} named case(s) not written yet\n")
        for case in absent:
            print(f"  [{case['stage']}] {case['path']}")
        return 0

    if opts.sites or opts.unattributed:
        show = by_item.get(0, []) if opts.unattributed else found
        head = "unattributed refusal site" if opts.unattributed else "refusal site"
        print(f"{len(show)} {head}(s)\n")
        current = ""
        for site in show:
            if site["file"] != current:
                current = site["file"]
                print(f"  {current}")
            print(f"    :{site['line']}  {site['message'][:110]}")
        if opts.unattributed and show:
            print("\nEach is a hole nobody scheduled or a decision nobody wrote down. Both are the")
            print("goal's business before the code is -- loop-goal.md § What \"no holes\" means.")
        return 0

    print(f"{len(found)} refusal site(s) across {len(SOURCES)} crate(s), "
          f"{len(guards)} guarded (--guarded), "
          f"{len(scheduled)} scheduled item(s), "
          f"{len(absent)} of {len(cases)} named case(s) still to write\n")

    print("  ITEMS WITH A REFUSAL SITE STILL STANDING")
    live = [i for i in scheduled if by_item.get(i["n"])]
    for item in live:
        print(f"    {item['n']:>3}  {len(by_item[item['n']]):>2} site(s)  {item['title'][:88]}")
    if not live:
        print("    none -- every scheduled item's files are clean")

    orphan = by_item.get(0, [])
    print(f"\n  UNATTRIBUTED: {len(orphan)} site(s) in files no item anchors")
    for site in orphan[:12]:
        print(f"    {site['file']}:{site['line']}  {site['message'][:88]}")
    if len(orphan) > 12:
        print(f"    ... and {len(orphan) - 12} more (--unattributed)")

    print(f"\n  {len(absent)} named case(s) still to write (--cases)")
    print("\n  python tools/holes.py --item N     one item in full")
    return 0


if __name__ == "__main__":
    sys.exit(main())
