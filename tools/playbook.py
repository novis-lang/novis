#!/usr/bin/env python3
"""Pick the playbook bullets a goal actually needs, and report the ones that have gone stale.

`docs/agent/playbook.md` is append-mostly by decision -- every trap a session writes down is
charged to every session after it -- and three things bound what that costs. A bullet has a shape
(docs/agent/conventions.md § *A playbook bullet*: three sentences, about 400 B) and a weight
`session.py --wrap` refuses past its `PLAYBOOK_BULLET_MAX`; every bullet declares what retires it,
and the wrap deletes it the day that holds; and `orient.py` slices the file twice -- by the goal's
`[context] playbook`, then again by the paths the session's own item names -- so an entry there
may name **one bullet** rather than a section: `"Tooling > a whole decision record"`. Run `--check`
for the live figures; a number quoted in prose is stale the week after it is written.

What was missing is any cheap way to decide *which* bullets a given file set implies. That is this
script.

    python tools/playbook.py                       # every section and bullet, one line, with sizes
    python tools/playbook.py --show <selector>     # one bullet or section, as orient.py prints it
    python tools/playbook.py --match <term>...     # bullets ranked against paths / crates / words
    python tools/playbook.py --manifest <term>...  # the same, as a paste-ready `playbook = [...]`
    python tools/playbook.py --goal                # --manifest driven by loop-goal.toml's modules
    python tools/playbook.py --check               # expiry, stale paths, colliding selectors, sizes (CI)
    python tools/playbook.py --retire              # delete every bullet whose retirement condition holds, and the manifest lines that named only it
    python tools/playbook.py --dupes               # bullets that already say what another says

EVERY BULLET DECLARES WHAT RETIRES IT

An append-mostly file has no natural way out, so the way out is declared at the way in. The last
thing in a bullet is a trailer naming the condition under which the bullet is deleted:

    - **The lead-in.** The trap, as before ... and the last line ends with [until: <kind> <arg>]

Five kinds. The first four are mechanical -- `--check` tests them against the tree and `--retire`
deletes the bullets whose condition holds -- and the fifth is the one for everything else:

    [until: test <fn_name>]           a Rust test with this exact name exists (`fn <name>` under
                                      crates/, tests/ or benches/) -- for a hole a test will pin
    [until: exists <path>[:<needle>]] the repo-rooted path exists, and holds the needle if one is
                                      given -- for "X does not exist yet, so work around it"
    [until: gone <path>[:<needle>]]   the path is gone, or no longer holds the needle -- for a
                                      trap that dies with a flag, a table, a file
    [until: rule <topic>/<slug>]      `docs/rules/<topic>/<slug>.md` exists -- for "this is
                                      undecided, do not assume"
    [until: reviewed <YYYY-MM-DD>]    no condition; the date a reader last confirmed the bullet is
                                      still true. `--check` lists a bullet whose date is more
                                      than REVIEW_DAYS old as owed a re-read; re-reading it and
                                      finding it true bumps the date, finding it false deletes it

A needle is a plain substring, never a regex, and may not hold `]`. The whole trailer sits on one
line, past the prose's wrap column if that is what it takes: an argument carrying a line break is an
argument no path, needle, name or date can equal, and `--check` reports one as malformed rather than
reading it. The same trailer, with the
same kinds, is what `docs/agent/guard-name-debt.md`'s bullets, `docs/agent/carried-gaps.md`'s
§ *Unowned* bullets and `docs/agent/carried-refusals.md`'s numbered entries carry; a row in
carried-gaps' § *Owned* table declares through its *Owner* column instead, and `--check` flags a
row whose owner has been retired in the goals directory without the row being struck. This paragraph is
the only home of the syntax: `session.py --wrap` refuses a `## playbook:` bullet without a
trailer and points here, and `session-prompt.md` and `commands.md` point here rather than
restating it.

An expired bullet is deleted, not archived and not commented out, because `git log -S` over this
file holds every one of them for nothing. Unit C7 of the docs migration declared the 1,059
bullets standing on 2026-09-06 and deleted the ones already expired in the same transaction.

A term is a path (`crates/nvs-ir/src/lower/expr.rs`), a crate (`nvs-ir`), a tool (`peek.py`) or a
plain word. A path is expanded to the things a bullet would actually spell -- the posix path, the
file name, the stem, the crate in both `nvs-ir` and `nvs_ir` spellings -- so naming the handoff's
own file set is enough. Those spellings are not equal: the ones that *name* the term rank, and the
ones derived by stripping an extension off it only break ties, because `server.rs` stripped is an
ordinary English word. `spellings` holds that split and `score` applies it.

**This script never appends to the playbook.** Appending a bullet has one home already, and it is
`session.py`'s `## playbook: <heading>` section, which keeps the whole session tail at one call.
A second way to add one would be a second thing to keep in agreement. The one write this script
does make is `--retire`, and it only ever removes: a bullet whose declared condition holds is
mechanically dead, and a second reader deciding that again is the cost the trailer exists to end.
It removes the goal manifests' `playbook` lines that resolved to that bullet alone in the same
pass, because `chain.py --check` refuses a selector that reaches nothing and every goal's floor
runs that check -- a bullet retired by a wrap and left named by the live manifest held the loop
for a hand once, at a DONE claim, over a line that had outlived the trap it fetched.

Beyond the trailers, `--check` and `--dupes` are the two pruning signals an append-mostly file can
have. A bullet naming a path that is no longer in the tree is describing a trap someone already
closed; a bullet sharing most of its three-word runs with another is a trap that was written down
twice. **Five separate sessions wrote the `wsl.exe` path-mangling bullet, one each, in five
wordings**, and every copy was charged to every session afterwards -- `--check` could see two of
them, because their lead-ins happened to collide as selectors, and was blind to the other three.

Both report; neither deletes, and neither exits non-zero over a size (docs/agent/doc-style.md
§ *Length targets*) or over a stale-looking path. Two bullets about one file are often two
different traps, and a path a bullet quotes may be gone precisely because the trap was closed --
only a reader can tell either way. That is exactly the judgement the `gone` kind lets a reader
make once, at writing time, instead of at every pass.

Once a reader has told, two tables below record it, and both work the same way: `DELIBERATE_STALE`
keyed by the exact `(selector, path)` pair, `DELIBERATE_DISTINCT` by the exact `(selector,
selector)` one. Those bullets still print, under their own heading, but out of the list `loop.py`'s
checkpoint reads -- because a bullet whose whole subject is a path that is gone, or a pair whose
overlap is one bullet citing the other, keeps that signal raised forever, and a signal that cannot
clear schedules an optimization pass whether or not anything drifted.

**Two findings gate, and `--check` exits 1 on them: a selector that does not resolve to exactly
one bullet, and a bullet with no trailer or a malformed one.** Neither is a judgement call.
`orient.py` fetches a trap by selector and a goal's `[context] playbook` names bullets that way,
so a lead-in two bullets share, or one no selector reaches at all, is a trap the loop silently
cannot deliver -- the session never learns it existed. And a bullet that declares nothing is one
the file can never let go of, which is the whole failure this tool's expiry half exists to end.
CI's `docs` job runs this for those two findings alone; an expired bullet and an overdue re-read
are reported, never refused, because the fix for the first is `--retire` and for the second a
reader.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import goals as goalsmod  # noqa: E402  -- the chain's one reader
import orient as orientmod  # noqa: E402  -- bullet parsing lives there and is not reimplemented

ROOT = Path(__file__).resolve().parent.parent
PLAYBOOK = ROOT / "docs" / "agent" / "playbook.md"
GOAL_TOML = ROOT / "docs" / "agent" / "loop-goal.toml"
HANDOFF = ROOT / "docs" / "agent" / "handoff.md"
GUARD_DEBT = ROOT / "docs" / "agent" / "guard-name-debt.md"
CARRIED_GAPS = ROOT / "docs" / "agent" / "carried-gaps.md"
CARRIED_REFUSALS = ROOT / "docs" / "agent" / "carried-refusals.md"

#: The trailer every bullet ends with. The module doc above is the syntax's one home.
EXPIRY = re.compile(r"\[until:\s*(test|exists|gone|rule|reviewed)\s+([^\]]+?)\s*\]\s*$")
#: Anything that looks like a trailer but did not parse as one, so a typo is a finding and not a
#: bullet that silently declares nothing.
EXPIRY_LIKE = re.compile(r"\[until:[^\]]*\]?\s*$")
#: How old a `reviewed` date may be before `--check` lists the bullet as owed a re-read. Sixty
#: days: the file grew from 86 bullets to over a thousand in a fortnight, so a claim two months
#: old has outlived most of the tree it was written against.
REVIEW_DAYS = 60

#: The other three append-mostly files and which of their blocks must declare. A block is a `- `
#: bullet or a `NNN. ` entry at column 0; the predicate takes the section heading it sits under
#: and the block's first line.
DECLARING = {
    PLAYBOOK: lambda head, first: True,
    GUARD_DEBT: lambda head, first: first.startswith("- ["),
    CARRIED_GAPS: lambda head, first: head == "Unowned",
    CARRIED_REFUSALS: lambda head, first: bool(re.match(r"^9\d\d\. ", first)),
}

#: A source path written into handoff prose, e.g. `crates/nvs-ir/src/lower/expr.rs:1876`.
HANDOFF_PATH = re.compile(r"\b((?:crates|tools|tests|benches|examples|fuzz)/[\w./-]+\.\w+)")

#: Directories a bullet names when it names a *tracked* path. Anything matching one of these is
#: checked against the tree by `--check`; anything else in backticks is prose, a symbol or a
#: command, and this script does not guess about those. `.loop/` and `.agent-tmp/` are left out
#: deliberately: both are runtime scratch, so a bullet naming `.loop/running` is describing an
#: artifact that exists only while a loop runs, not a file that has gone missing.
TREE_DIRS = ("crates/", "tools/", "docs/", "tests/", "benches/", "examples/", "fuzz/", ".github/")

#: A path in prose collects punctuation and a locator. Strip both before asking the disk. The
#: locator is any of `peek.py`'s target forms -- `:120-160`, `:120+30`, `:@sym`, `:re:pattern` --
#: because a bullet quoting one of those is naming a file that IS in the tree: `--check` reported
#: `docs/agent/loop-goal.toml:re:a_named_connection_is_memoized` as a missing path for as long as
#: it only knew about `:\d+`, and an optimization pass paid to re-derive that it was not.
PATH_TRIM = re.compile(r"(:re:.*|:@[\w:.-]+|:\d+([-+]\d+)?|[.,;:)\]'\"]+)$")

#: A bullet naming two siblings at once -- `crates/nvs-runtime/{src,tests}`, which is the spelling
#: the `grep` in the same sentence takes -- is naming both of them, and both have to be in the tree
#: for the trap to stand. Expand the comma-list and ask the disk about each: unexpanded, the brace
#: reads as a path nothing has, and that is a stale-path signal no pass can ever clear.
BRACES = re.compile(r"^([^{}]*)\{([^{}]+)\}([^{}]*)$")


def brace_expand(cand: str) -> list[str]:
    """`a/{b,c}/d` as the two paths it names; anything else unchanged."""
    match = BRACES.match(cand)
    if not match:
        return [cand]
    head, inner, tail = match.groups()
    return [f"{head}{part.strip()}{tail}" for part in inner.split(",") if part.strip()]

#: Bullets whose missing path is the whole point of the trap -- they quote a path that is gone, or
#: that was never right, *because that is what the bullet is about*. Keyed by the exact
#: `(selector, path)` pair, so any other path in the same bullet, and this path in any other
#: bullet, still reports normally.
#:
#: This exists because `loop.py`'s checkpoint fires an optimization pass unless the list below says
#: `none`, and these two can never leave it: the trap they describe is the stale path. Three passes
#: in a row read them and wrote down that they were deliberate, and the fourth was scheduled on
#: their account alone -- a signal that cannot clear is a constant, and it spends a pass whether or
#: not anything drifted. They are still printed, under their own heading, so the next reader sees
#: them without the loop paying to schedule that reader.
#:
#: Add an entry only after reading the bullet and recording the finding in `.loop/optimization/`.
#: An entry naming a bullet that no longer exists is reported rather than ignored.
DELIBERATE_STALE: dict[tuple[str, str], str] = {
    # The bullet's subject is that `check-links.py` resolves a crate path written without its
    # `crates/` prefix as a suffix against the repository root, so it has to spell the suffix that
    # nothing answers beside the real file it came from. Rewording it to drop the suffix would
    # delete the trap. Add a pair here only when a bullet's whole subject IS a path that is gone,
    # so `--check`'s stale-path signal can still reach `none`. Every key here is absent by
    # construction, which is `check-links.py`'s `MENTION_SUBJECT` in one sentence, so a new one
    # carries that marker on its own line or it turns the floor's link gate red.
    ("Tooling > a tool's prose", "tests/vectors.rs"):  # check-links:subject
        "the suffix `check-links.py` wrongly resolves to; the file is "
        "crates/nvs-stdlib/src/tests/vectors.rs, and the bullet names both because the "
        "relation between them is the trap",
}

#: The default `--dupes` floor, and the only threshold at which `DELIBERATE_DISTINCT` is audited.
DUPES_FLOOR = 0.22

#: Bullet pairs a reader has compared and found to be two different traps. Keyed by the exact
#: `(selector, selector)` pair, sorted, so either bullet paired with any third one still reports.
#:
#: The same argument as `DELIBERATE_STALE` above, for the other pruning signal: `loop.py`'s
#: checkpoint fires an optimization pass unless `--dupes` prints `none at this threshold`, so a
#: pair that overlaps for a legitimate reason schedules a pass every checkpoint for as long as both
#: bullets stand. Overlap is over three-word runs, so one bullet CITING another's fact -- the
#: ordinary way a trap points at its follow-up step -- reads exactly like the same trap written
#: twice, and no rewording can fix it without breaking the cross-reference.
#:
#: They are still printed, under their own heading, so the next reader sees the pair without the
#: loop paying to schedule that reader. Add an entry only after reading both bullets and recording
#: the finding in `.loop/optimization/`. An entry whose pair no longer overlaps is reported rather
#: than ignored.
DELIBERATE_DISTINCT: dict[tuple[str, str], str] = {
    ("Tooling > a loop-goal.toml check can name a test in",
     "Tooling > docs/agent/loop-goal.toml and"):
        "the first is a misfiled check whose `args` is wrong; the second is the goals/ copy-back "
        "drift. The first ends by citing the second as its follow-up step, and that citation is "
        "the whole overlap.",
}


def nbytes(text: str) -> int:
    return len(text.encode("utf-8"))


def report_growth(indent: str = "") -> None:
    """How fast the file is growing, and whether anything is ever taken out.

    THE THIRD SIGNAL, and the one the other two structurally cannot give. `--check` finds bullets
    whose paths are gone; `--dupes` finds bullets that restate one another. Both were reading
    `none` on 2026-09-06 while the file went 644 KB -> 860 KB in four days, because neither can
    see a bullet that is correct, unique, and no longer worth 818 bytes to every future session.
    An append-mostly file with no expiry rule has exactly one honest measurement -- the rate --
    and this is it.

    `git log --numstat` over the one file, in one call: no `git show` per revision, so this stays
    cheap enough to sit inside `--check`.

    It reports and never gates. A rate is not a defect; it is the number a person weighs when
    deciding whether the next pass prunes. The enforceable brake is `orient.py`'s `PROMOTED_WHOLE`,
    which bounds what any one session PAYS regardless of what the file holds."""
    try:
        out = subprocess.run(
            ["git", "log", "--numstat", "--format=%H %at", "--", str(PLAYBOOK.relative_to(ROOT))],
            cwd=ROOT, capture_output=True, text=True, timeout=30,
        ).stdout
    except (OSError, subprocess.SubprocessError):
        return
    revs, added, removed, stamps = 0, 0, 0, []
    for line in out.split("\n"):
        parts = line.split()
        if len(parts) == 2 and len(parts[0]) == 40:
            revs += 1
            try:
                stamps.append(int(parts[1]))
            except ValueError:
                pass
        elif len(parts) == 3 and parts[0].isdigit() and parts[1].isdigit():
            added += int(parts[0])
            removed += int(parts[1])
    if revs < 2 or not stamps:
        return
    days = max((max(stamps) - min(stamps)) / 86400.0, 1e-9)
    size = nbytes(read())
    print(f"\n{indent}== HOW FAST THIS FILE IS GROWING  (git log --numstat, whole history)")
    print(f"{indent}  {size:,} B now, over {revs} commit(s) and {days:.1f} day(s)")
    print(f"{indent}  +{added:,} line(s) added, -{removed:,} removed "
          f"-- {added / max(removed, 1):.0f} added for every 1 taken out")
    print(f"{indent}  {(added - removed) / days:,.0f} net line(s) a day")
    print(f"{indent}")
    print(f"{indent}  Neither of the signals above can fall while this rises: they find bullets")
    print(f"{indent}  that are WRONG or DUPLICATED, and a file grows on bullets that are neither.")
    print(f"{indent}  What bounds a SESSION's share is orient.py's PROMOTED_WHOLE, not this rate.")
    print(f"{indent}  Read this when deciding whether a pass should prune, and nothing else.")


def read() -> str:
    return PLAYBOOK.read_text(encoding="utf-8")


def sections(text: str) -> list[str]:
    return [m.group(1) for m in re.finditer(r"^## (.+)$", text, flags=re.M)]


def all_bullets(text: str) -> list[dict]:
    """Every bullet, with the section it is in and a selector that resolves to it alone."""
    found = []
    for head in sections(text):
        for name, body in orientmod.bullets(text, head):
            found.append({"section": head, "name": name, "body": body, "bytes": nbytes(body)})
    for b in found:
        b["selector"] = f"{b['section']} > {shortest_key(b, found)}"
    return found


def shortest_key(bullet: dict, every: list[dict]) -> str:
    """The fewest words of a bullet's lead-in that name it and nothing else in its section.

    Uniqueness is tested as a *substring* of every neighbour's normalized lead-in, which is
    stricter than `slice_bullets`' match on a lead-in's opening words: a key no neighbour contains
    is one no neighbour opens with either, so the selector resolves to this bullet alone. The
    stricter test is kept because it leaves every selector already written into a manifest, or
    keyed in `DELIBERATE_*` above, spelled the way it was emitted."""
    peers = [b for b in every if b["section"] == bullet["section"] and b is not bullet]
    words = orientmod.normalize(bullet["name"]).split()
    for n in range(2, len(words) + 1):
        key = " ".join(words[:n])
        if not any(key in orientmod.normalize(p["name"]) for p in peers):
            return key
    return orientmod.normalize(bullet["name"])


# ------------------------------------------------------------------------------ expiry


def declaration(body: str) -> tuple[str, str] | None:
    """The `[until: kind arg]` a block ends with, or None when it declares nothing.

    `session.py --wrap` asks this of every `## playbook:` bullet before appending it, so the
    trailer's grammar has one reader and the wrap and the check cannot disagree about it.

    A trailer wrapped over two lines declares nothing. Its argument then holds a newline, which no
    path, test name, rule slug or date can, and which a needle read out of a file will never match
    -- so a wrapped `gone` holds the day it is written and `--retire` deletes a bullet whose trap
    is still live. `wrapped` is what turns that into a finding."""
    m = EXPIRY.search(body.rstrip())
    return (m.group(1), m.group(2).strip()) if m and "\n" not in m.group(2) else None


def wrapped(body: str) -> bool:
    """Whether the block ends with a trailer broken across a line, which `declaration` refuses."""
    m = EXPIRY.search(body.rstrip())
    return bool(m and "\n" in m.group(2))


def blocks(text: str) -> list[dict]:
    """Every `- ` bullet and every `NNN. ` entry at column 0, with its line span and section.

    The playbook's own bullets come from `orient.bullets` (one parser, one home); this is the
    shape the other three files share with it, so their blocks can be checked and retired by the
    same code. A block runs to the next block at column 0, the next heading, or the end."""
    out, cur, head = [], None, ""
    lines = text.split("\n")
    for i, line in enumerate(lines):
        starts = re.match(r"^(- |\d+\. )", line)
        if re.match(r"^#{1,6}\s", line):
            if cur:
                out.append(cur)
            cur = None
            head = re.sub(r"^#+\s*", "", line).strip()
            continue
        if starts:
            if cur:
                out.append(cur)
            cur = {"section": head, "first": line, "start": i, "end": i}
        elif cur is not None and not line.strip():
            # A blank line ends a block only when what follows is not indented continuation:
            # carried-refusals' entries carry indented paragraphs past blank lines.
            nxt = lines[i + 1] if i + 1 < len(lines) else ""
            if nxt and not nxt.startswith((" ", "\t")) and not re.match(r"^(- |\d+\. )", nxt):
                out.append(cur)
                cur = None
        elif cur is not None:
            cur["end"] = i
    if cur:
        out.append(cur)
    for b in out:
        b["body"] = "\n".join(lines[b["start"]:b["end"] + 1]).rstrip()
        b["lead"] = re.sub(r"^(- (?:\[.\] )?|\d+\. )", "", b["first"])[:70]
    return out


_TEST_NAMES: set[str] | None = None


def test_names() -> set[str]:
    """Every `fn <name>` in a `.rs` file git tracks, read once."""
    global _TEST_NAMES
    if _TEST_NAMES is None:
        _TEST_NAMES = set()
        try:
            out = subprocess.run(
                ["git", "grep", "-h", "-o", "-E", r"\bfn [A-Za-z_][A-Za-z0-9_]*", "--", "*.rs"],
                cwd=ROOT, capture_output=True, text=True, timeout=60,
            ).stdout
        except (OSError, subprocess.SubprocessError):
            out = ""
        for line in out.split("\n"):
            if line.startswith("fn "):
                _TEST_NAMES.add(line[3:].strip())
    return _TEST_NAMES


def holds(kind: str, arg: str, today: date) -> tuple[bool | None, str]:
    """Whether a declared condition holds today, and a word on why.

    Returns (True, why) when the block has expired, (False, why) when it stands, and (None, why)
    when the declaration cannot be evaluated -- a malformed date, an argument the kind cannot
    read -- which `--check` reports as a finding rather than guessing either way."""
    if kind == "reviewed":
        try:
            when = date.fromisoformat(arg)
        except ValueError:
            return None, f"`{arg}` is not a YYYY-MM-DD date"
        age = (today - when).days
        if age > REVIEW_DAYS:
            return False, f"reviewed {age} days ago -- owed a re-read"
        return False, f"reviewed {age} days ago"
    if kind == "test":
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", arg):
            return None, f"`{arg}` is not a test function name"
        return arg in test_names(), f"fn {arg} {'exists' if arg in test_names() else 'is not in the tree'}"
    if kind == "rule":
        p = ROOT / "docs" / "rules" / f"{arg}.md"
        return p.exists(), f"docs/rules/{arg}.md {'exists' if p.exists() else 'does not exist'}"
    if kind in ("exists", "gone"):
        path, _, needle = arg.partition(":")
        path = path.strip().replace("\\", "/")
        if not path or any(c in path for c in "*?<>"):
            return None, f"`{path}` is not a repo path"
        p = ROOT / path
        present = p.exists()
        if present and needle:
            try:
                present = needle in p.read_text(encoding="utf-8", errors="replace")
            except OSError:
                present = False
        what = f"{path}{' holds ' + repr(needle) if needle and present else ''}" if present else (
            f"{path} does not exist" if not p.exists() else f"{path} no longer holds {needle!r}")
        return (present if kind == "exists" else not present), what
    return None, f"unknown kind {kind!r}"


def chain_retired() -> set[str]:
    """The slugs of the retired goals -- the owners carried-gaps may no longer name.

    Retired is the goal's `.toml` being gone, and `tools/goals.py` is what reads that. A slug and
    not a number, because the Owner cell holds a slug: a number there would name whichever goal had
    moved into it by the time anyone read the table.
    """
    return {g.slug for g in goalsmod.load() if g.retired}


def owned_rows(text: str) -> list[tuple[int, str, str]]:
    """carried-gaps' § *Owned* table: (line index, gap cell, owner cell)."""
    rows = []
    section = orientmod.slice_section(text, "Owned") or ""
    if not section:
        return rows
    offset = text.split("\n").index(section.split("\n")[0])
    for i, line in enumerate(section.split("\n")):
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if line.startswith("|") and len(cells) >= 2 and cells[0] not in ("Gap", "---") and not set(cells[0]) <= {"-"}:
            rows.append((offset + i, cells[0], cells[1]))
    return rows


def expiry_report(today: date | None = None) -> tuple[list[dict], list[dict], list[dict], list[dict]]:
    """Over the four files: (expired, owed a re-read, undeclared or malformed, owner-retired rows)."""
    today = today or date.today()
    expired, owed, bad, rows = [], [], [], []
    for path, must in DECLARING.items():
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8")
        rel = path.relative_to(ROOT).as_posix() if path.is_relative_to(ROOT) else path.as_posix()
        for b in blocks(text):
            decl = declaration(b["body"])
            where = {"file": rel, "path": path, "line": b["start"] + 1, "lead": b["lead"],
                     "start": b["start"], "end": b["end"]}
            if decl is None:
                if wrapped(b["body"]):
                    bad.append({**where, "why": "the trailer is broken across two lines, so its "
                                "argument holds a newline no path, needle, name or date can -- put "
                                "the whole `[until: ...]` on one line, past the wrap column if need be"})
                elif EXPIRY_LIKE.search(b["body"]):
                    bad.append({**where, "why": "the trailer does not parse -- it is "
                                "`[until: <kind> <arg>]`, kind one of test, exists, gone, rule, reviewed"})
                elif must(b["section"], b["first"]):
                    bad.append({**where, "why": "no `[until: ...]` trailer"})
                continue
            kind, arg = decl
            ok, why = holds(kind, arg, today)
            entry = {**where, "kind": kind, "arg": arg, "why": why}
            if ok is None:
                bad.append(entry)
            elif ok:
                expired.append(entry)
            elif kind == "reviewed" and "owed" in why:
                owed.append(entry)
        if path == CARRIED_GAPS:
            gone = chain_retired()
            for line, gap, owner in owned_rows(text):
                slug = owner.strip().strip("`")
                if slug in gone:
                    rows.append({"file": rel, "line": line + 1, "lead": gap[:70],
                                 "why": f"owner goal `{slug}` is retired; close the gap or strike "
                                        f"the owner"})
    return expired, owed, bad, rows


def report_expiry(today: date | None = None) -> tuple[int, int]:
    """Print the four expiry findings; return (expired count, undeclared-or-malformed count)."""
    expired, owed, bad, rows = expiry_report(today)
    declared = sum(1 for p in DECLARING if p.exists() for b in blocks(p.read_text(encoding="utf-8"))
                   if declaration(b["body"]))
    print("== BULLETS WHOSE RETIREMENT CONDITION HOLDS  (delete them: `python tools/playbook.py --retire`)")
    for e in expired:
        print(f"  {e['file']}:{e['line']}  {e['lead']}")
        print(f"      [until: {e['kind']} {e['arg']}]  -- {e['why']}")
    if not expired:
        print(f"  none -- every one of the {declared} declared condition(s) still stands")
    else:
        print(f"\n  {len(expired)} bullet(s). Each is mechanically dead: the thing it waited for is on disk,")
        print("  or the thing it was about is gone. `git log -S` keeps the text; the file need not.")

    print(f"\n== BULLETS OWED A RE-READ  (`reviewed` more than {REVIEW_DAYS} days ago)")
    for e in owed:
        print(f"  {e['file']}:{e['line']}  {e['lead']}  -- {e['why']}")
    if not owed:
        print("  none")
    else:
        print(f"\n  {len(owed)} bullet(s). Read each; still true bumps its date, no longer true deletes it.")

    print("\n== CARRIED-GAPS ROWS WHOSE OWNER WENT GREEN WITHOUT CLOSING THEM")
    for r in rows:
        print(f"  {r['file']}:{r['line']}  {r['lead']}  -- {r['why']}")
    if not rows:
        print("  none -- every carried-gaps owner is live or struck")

    print("\n== BULLETS THAT DECLARE NOTHING, OR DECLARE IT WRONGLY")
    for e in bad:
        print(f"  {e['file']}:{e['line']}  {e['lead']}")
        print(f"      {e['why']}")
    if not bad:
        print("  none -- every bullet ends with a trailer this tool can read")
    return len(expired), len(bad)


def run_closes(slug: str) -> int:
    """`--closes SLUG`: every `carried-gaps.md` § *Owned* row that still names the goal, and 1 if
    there is one.

    The other half of `tools/owners.py --closes`, over the index rather than the module docs, and
    asked for the same reason: the retired-owner rows `--check` prints can only appear once the
    goal is retired, which is after it was reached, so `tools/loop.py`'s `owner_gate` asks this of
    the goal by name on the sweep that would reach it. A row leaves the table when its gap is
    closed and the row deleted, or when the owner is struck for a reason the row states.
    """
    text = CARRIED_GAPS.read_text(encoding="utf-8") if CARRIED_GAPS.exists() else ""
    rel = CARRIED_GAPS.relative_to(ROOT).as_posix()
    rows = [(line + 1, gap) for line, gap, owner in owned_rows(text)
            if owner.strip().strip("`") == slug]
    if not rows:
        print(f"playbook.py: goal `{slug}` owns no {rel} row")
        return 0
    for line, gap in rows:
        print(f"  {rel}:{line}  {gap[:70]}")
    print(f"playbook.py: goal `{slug}` still owns {len(rows)} {rel} row(s). A goal is reached when "
          f"each gap is closed and its row deleted, or its owner struck for a reason the row "
          f"states; a tag is not a build.")
    return 1


def run_retire(dry: bool) -> int:
    """The `--retire` flag: refuse while a declaration cannot be read, otherwise `retire`."""
    expired, _owed, bad, _rows = expiry_report()
    if bad:
        print(f"playbook.py: {len(bad)} bullet(s) declare nothing or declare it wrongly; `--check` "
              "names them. Nothing is retired while a declaration cannot be read.")
        return 1
    if not expired:
        print("playbook.py: no bullet's retirement condition holds; nothing to delete.")
        return 0
    retire(expired, dry)
    return 0


def retire(expired: list[dict], dry: bool) -> list[str]:
    """Delete every block in `expired`, file by file, then every goal manifest's `playbook` line
    that resolved only to a bullet that just went. Returns the files changed, repo-relative.

    The second half is what keeps a retirement from halting the loop: a goal's `[context]
    playbook` names its bullets by lead-in, `chain.py --check` refuses a selector that reaches
    nothing, and that check is on every goal's floor. So a bullet retired by one wrap and left
    named by the live manifest was a DONE claim the driver held for a hand -- the selector had
    outlived the trap it fetched, and nothing but a reader's grep connected the two files."""
    by_file: dict[Path, list[dict]] = {}
    for e in expired:
        by_file.setdefault(e["path"], []).append(e)
    before = read()
    after = before
    changed: list[str] = []
    for path, entries in by_file.items():
        lines = path.read_text(encoding="utf-8").split("\n")
        for e in sorted(entries, key=lambda x: -x["start"]):
            print(f"  retire  {e['file']}:{e['line']}  {e['lead']}  -- {e['why']}")
            del lines[e["start"]:e["end"] + 1]
            # Collapse the blank line a deleted block leaves behind, so two sections never end
            # up separated by two.
            if 0 < e["start"] < len(lines) and not lines[e["start"]].strip() and not lines[e["start"] - 1].strip():
                del lines[e["start"]]
        text = "\n".join(lines)
        if path == PLAYBOOK:
            after = text
        if not dry:
            path.write_text(text, encoding="utf-8", newline="\n")
        changed.append(entries[0]["file"])
    pruned = prune_manifests(before, after, dry)
    verb = "would delete" if dry else "deleted"
    print(f"\nplaybook.py: {verb} {len(expired)} bullet(s) across {len(by_file)} file(s)"
          + (f" and {len(pruned)} manifest line(s) that named nothing else" if pruned else "")
          + ". Commit is the caller's; the message names what expired.")
    return changed + sorted({p for p, _sel in pruned})


#: One entry of a `playbook = [ ... ]` list as the goal files write it: a TOML string on a line
#: of its own, a trailing comma and comment allowed. Inline lists are not read -- the scaffold
#: only ever writes the empty one, and `chain.py --check` still reports a selector in one.
SELECTOR_LINE = re.compile(r"""^\s*('[^']*'|"(?:[^"\\]|\\.)*")\s*,?\s*(?:#.*)?$""")


def prune_manifests(before: str, after: str, dry: bool) -> list[tuple[str, str]]:
    """Drop, from every goal's `[context]`, each `playbook` selector that resolved against
    `before` and resolves to nothing against `after`. Returns `(file, selector)` per line dropped.

    Only the difference is dropped. A selector that already reached nothing is `chain.py
    --check`'s finding and stays for a reader; one that still opens another bullet's lead-in --
    a family named with `*`, or a lead-in the retired bullet shared -- keeps fetching that, and
    stays too. A comment run left with nothing under it before the closing `]` goes with its
    lines, so a manifest never ends in a comment about bullets it no longer names.

    `docs/agent/loop-goal.toml` is swept with the goals: it is the live goal's installed copy,
    the one `orient.py` builds the pack from and the driver runs the sweep from, and a session
    edits it in place, so it can name a selector the goal's own file no longer does."""
    import tomllib
    dropped: list[tuple[str, str]] = []
    manifests = [g.toml for g in goalsmod.load() if not g.retired]
    if GOAL_TOML.is_file():
        manifests.append(GOAL_TOML)
    for toml in manifests:
        lines = toml.read_text(encoding="utf-8").split("\n")
        keep: list[str] = []
        went: list[str] = []
        in_list = False
        comments_since_kept = 0
        last_went = False
        for line in lines:
            if not in_list:
                keep.append(line)
                in_list = re.match(r"^\s*playbook\s*=\s*\[\s*(?:#.*)?$", line) is not None
                comments_since_kept, last_went = 0, False
                continue
            stripped = line.strip()
            if stripped == "]":
                if last_went and comments_since_kept:
                    del keep[len(keep) - comments_since_kept:]
                keep.append(line)
                in_list = False
                continue
            m = SELECTOR_LINE.match(line)
            if m is None:
                keep.append(line)
                if stripped.startswith("#"):
                    comments_since_kept += 1
                continue
            selector = tomllib.loads(f"x = {m.group(1)}")["x"]
            was, _ = orientmod.slice_bullets(before, selector)
            _now, complaint = orientmod.slice_bullets(after, selector)
            if was and complaint:
                went.append(selector)
                last_went = True
                continue
            keep.append(line)
            comments_since_kept, last_went = 0, False
        if not went:
            continue
        rel = toml.relative_to(ROOT).as_posix()
        text = "\n".join(keep)
        try:
            tomllib.loads(text)
        except tomllib.TOMLDecodeError as err:
            print(f"  keep    {rel}  -- dropping {len(went)} selector(s) leaves it unreadable "
                  f"({err}); left for a hand")
            continue
        for selector in went:
            print(f"  drop    {rel}  {toml_str(selector)}  -- named only a bullet retired above")
            dropped.append((rel, selector))
        if not dry:
            toml.write_text(text, encoding="utf-8", newline="\n")
    return dropped


# ------------------------------------------------------------------------------ matching


#: Path segments that carry no information about *which* work a bullet is about, because every
#: path in the tree has them. Without this, `crates/nvs-diagnostics/src/lib.rs` expands to `src`
#: and `lib`, and every bullet that mentions any Rust file at all scores a hit -- which is how a
#: first run of `--goal` proposed 50 of 86 bullets and called it narrowing.
GENERIC = {"src", "lib", "mod", "main", "crates", "tests", "docs", "tools", "benches", "rs", "md"}


def spellings(term: str) -> tuple[set[str], set[str]]:
    """One query term -> the `(strong, weak)` spellings a bullet might use for it.

    A **strong** spelling names the term itself: the path as written, a file's basename *with* the
    extension, and the crate directory it sits in. A **weak** one is derived by stripping something
    off, and stripping is what turns a path into an ordinary English word --
    `crates/nvs-lsp/src/server.rs` yields `server`, which appears in bullets about `nvs-server`,
    about `nvs-db`'s wire and about the loop driver, none of which is the LSP server. Measured on
    goal `lsp-server`'s stage-4 item, that stem promoted nine bullets to full text and **not one of them**
    matched `nvs-lsp`: 4,992 bytes of every session's pack, spent on the wrong crate.

    A directory's last segment is weak for the same reason, since it has no extension to keep it
    specific: `docs/rules/security` ends in `security`, which a bullet about anything guarded uses.

    A term with no `/` is a word the caller typed rather than one this derived, so all of its
    spellings are strong -- `--match hover` must still find the bullets about hover, and there is
    nothing else for it to match on.

    `score` is where the two are told apart; this only says which is which.
    """
    t = term.strip().replace("\\", "/").lower()
    if "/" not in t:
        strong = {t, t.replace("-", "_"), t.replace("_", "-")}
        return _usable(strong), set()

    parts = [p for p in t.split("/") if p]
    strong = {t}                                             # the path
    if "." in parts[-1]:
        strong.add(parts[-1])                                # expr.rs, never a directory's name
    weak = {parts[-1].rsplit(".", 1)[0]}                     # expr -- also an English word
    if len(parts) >= 2 and parts[0] == "crates":
        strong.add(parts[1])                                 # nvs-ir
        strong.add(parts[1].replace("-", "_"))               # nvs_ir
    if len(parts) >= 2:
        weak.add(parts[-2])                                  # lower
    return _usable(strong), _usable(weak) - _usable(strong)


def _usable(names: set[str]) -> set[str]:
    """A one- or two-letter fragment matches everything; so does a segment every path has."""
    return {x for x in names if len(x) > 2 and x not in GENERIC}


def expand(term: str) -> set[str]:
    """Every spelling, strong and weak together. `spellings` is what ranking uses."""
    strong, weak = spellings(term)
    return strong | weak


def toml_str(s: str) -> str:
    """One selector as a TOML string that parses.

    A lead-in is prose, so a key sliced from one can hold a `"` (a bullet opening with a quoted
    literal) or a `\\` (one naming `Core\\Path`). A TOML *literal* string takes both verbatim,
    which is why it is the default here; only a key that also holds a `'` needs the escaping
    form. Emitting a bare basic string looked right and produced a manifest that would not
    parse."""
    if "'" not in s:
        return f"'{s}'"
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def score(bullet: dict, terms: list[str]) -> tuple[int, list[str]]:
    """How many distinct query terms a bullet mentions, and which ones.

    **A weak spelling counts only in a bullet some term already matched strongly.** `spellings`
    says why the distinction exists; this is the rule it buys. A bullet that names the crate or the
    file scores its stems too, because there the stem is about the same thing; a bullet that has
    only the stem is using the word in its ordinary sense and scores nothing.

    The alternative -- dropping weak spellings outright -- loses the bullets that name a module by
    its bare stem (`expr`, `lower`), which is how this file often writes them.
    """
    hay = (bullet["body"] + " " + bullet["section"]).lower()
    strong = [t for t in terms if any(v in hay for v in spellings(t)[0])]
    if not strong:
        return 0, []
    weak = [t for t in terms
            if t not in strong and any(v in hay for v in spellings(t)[1])]
    return len(strong) + len(weak), strong + weak


def goal_terms() -> list[str]:
    """The current goal's own file set, out of `[context] modules`."""
    if not GOAL_TOML.exists():
        return []
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover -- 3.11+ everywhere this runs
        return []
    ctx = (tomllib.loads(GOAL_TOML.read_text(encoding="utf-8")).get("context") or {})
    return [str(m).rstrip("*/") for m in ctx.get("modules", [])]


def next_group_files() -> list[str]:
    """The file set `handoff.md`'s `## Next group` names, as paths.

    The manifest is goal-scoped and the handoff is session-scoped, so the two drift apart every
    time the work moves to a different crate. That drift used to be invisible because naming
    whole sections covered every file set by accident; a bullet-level manifest makes it bite."""
    if not HANDOFF.exists():
        return []
    text = HANDOFF.read_text(encoding="utf-8")
    m = re.search(r"^## Next group.*?(?=^## |\Z)", text, flags=re.M | re.S)
    if not m:
        return []
    seen: dict[str, None] = {}
    for path in HANDOFF_PATH.findall(m.group(0)):
        seen.setdefault(path, None)
    return list(seen)


def current_manifest() -> list[str]:
    if not GOAL_TOML.exists():
        return []
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover
        return []
    ctx = (tomllib.loads(GOAL_TOML.read_text(encoding="utf-8")).get("context") or {})
    return [str(x) for x in ctx.get("playbook", [])]


# ------------------------------------------------------------------------------ commands


def report_manifest(text: str, indent: str = "") -> None:
    """What `[context] playbook` currently costs, and whether every selector still resolves."""
    named = current_manifest()
    if not named:
        print(f"{indent}loop-goal.toml names no `[context] playbook`, so no trap is printed.")
        return
    cost, sections_named, dead = 0, 0, []
    for sel in named:
        hits, complaint = orientmod.slice_bullets(text, sel)
        if complaint:
            dead.append(sel)
            continue
        if ">" not in sel:
            sections_named += 1
        cost += sum(nbytes(h) for h in hits)
    whole = nbytes(text)
    print(f"{indent}[context] playbook names {len(named)} selector(s) "
          f"({sections_named} whole section(s)), costing {cost} B a session "
          f"-- {cost / whole * 100:.0f}% of the file.")
    if sections_named:
        print(f"{indent}A whole section grows every time a trap is written down. "
              "`--goal` proposes a bullet-level list.")
    for sel in dead:
        print(f"{indent}!! {sel!r} matches nothing -- orient.py will warn on it every session")


def run_gap(text: str, every: list[dict], floor: int) -> int:
    """Bullets the handoff's own next group implies that the manifest does not print.

    This is the check a narrowed manifest cannot do without: ranking against `[context] modules`
    answers "what does this GOAL touch", and the work in flight may have moved on. Measured the
    first time this ran, the manifest missed twelve bullets the next group implied -- including
    two on the exact `nvs-ir`/`nvs-codegen` path the handoff named -- because `modules` still
    listed a closed stage's stdlib file set."""
    files = next_group_files()
    if not files:
        print("playbook.py: handoff.md has no `## Next group` naming a path, so there is nothing "
              "to compare the manifest against.")
        return 0
    have = set(current_manifest())
    floor = max(1, min(floor, len(files)))
    print(f"handoff.md `## Next group` names {len(files)} file(s):")
    for f in files:
        print(f"  {f}")

    missing = []
    for b in every:
        n, hit = score(b, files)
        if n >= floor and b["selector"] not in have:
            missing.append((n, b))
    missing.sort(key=lambda x: (-x[0], x[1]["bytes"]))

    print(f"\nBullets those files imply ({floor}+ terms) that `[context] playbook` does NOT print:")
    if not missing:
        print("  none -- the manifest covers the work in flight")
        return 0
    for n, b in missing:
        print(f"  {n} term(s)  {b['bytes']:>5} B  {toml_str(b['selector'])},")
    print(f"\n  {len(missing)} bullet(s), {sum(b['bytes'] for _n, b in missing)} B. Add them to")
    print("  `[context] playbook`, or -- better -- fix `[context] modules` if it no longer")
    print("  describes the work, and re-run `--goal`. A manifest narrower than the work is")
    print("  the one way this tool can cost a session quality rather than save it tokens.")
    return 0


def run_index(text: str, every: list[dict]) -> int:
    total = nbytes(text)
    print(f"docs/agent/playbook.md: {total} bytes, {len(every)} bullets in "
          f"{len(sections(text))} sections")
    for head in sections(text):
        mine = [b for b in every if b["section"] == head]
        size = sum(b["bytes"] for b in mine)
        print(f"\n  ## {head}   {size} bytes, {len(mine)} bullets")
        for b in mine:
            print(f"    {b['bytes']:>5} B  {b['selector']}")

    print()
    report_manifest(text)
    print("\nA selector above is what `[context] playbook` takes verbatim. Naming the section")
    print("instead takes every bullet in it, including the ones written after this goal began.")
    return 0


def run_show(text: str, selector: str) -> int:
    hits, complaint = orientmod.slice_bullets(text, selector)
    if complaint:
        print(f"playbook.py: {complaint}")
        return 1
    for body in hits:
        print(body)
        print()
    print(f"-- {len(hits)} hit(s), {sum(nbytes(h) for h in hits)} bytes")
    if len(hits) > 1:
        print("   More than one: `--manifest` emits selectors that resolve to exactly one.")
    return 0


def run_match(text: str, every: list[dict], terms: list[str], as_manifest: bool,
              floor: int) -> int:
    if not terms:
        print("playbook.py: --match/--manifest needs at least one term, or use --goal")
        return 2
    # A floor above the number of terms can never be met, so `--match one-file` under the default
    # of 2 reported "no bullet mentions it" about a file three bullets name.
    floor = max(1, min(floor, len(terms)))
    scored, weak = [], []
    for b in every:
        n, hit = score(b, terms)
        if n >= floor:
            scored.append((n, b, hit))
        elif n:
            weak.append((n, b, hit))
    scored.sort(key=lambda x: (-x[0], x[1]["bytes"]))
    weak.sort(key=lambda x: (-x[0], x[1]["bytes"]))

    if not scored:
        print(f"playbook.py: no bullet mentions {floor}+ of {terms}. That is a real answer -- "
              "either this goal's file set has no trap written down yet, or --min is too high "
              f"({len(weak)} bullet(s) matched fewer).")
        return 0

    # Never a silent cut: what --min dropped is named, because a threshold that hides its own
    # tail reads as "there was nothing else" and the author cannot tell the two apart.
    def tail() -> None:
        if weak:
            print(f"\n# {len(weak)} further bullet(s) matched fewer than {floor} terms and are "
                  f"NOT listed ({sum(b['bytes'] for _n, b, _h in weak)} B). `--min 1` shows them.")

    if not as_manifest:
        print(f"{len(scored)} of {len(every)} bullets match {floor}+ of {terms}:\n")
        for n, b, hit in scored:
            print(f"  {n} term(s)  {b['bytes']:>5} B  {b['selector']}")
            print(f"             {' '.join(hit)}")
        print(f"\n  {sum(b['bytes'] for _n, b, _h in scored)} bytes in total. "
              "`--manifest` prints the same list as TOML.")
        tail()
        return 0

    total = sum(b["bytes"] for _n, b, _h in scored)
    whole = nbytes(text)
    print("# Bullets whose text names this goal's own file set, most specific first.")
    print(f"# {len(scored)} of {len(every)} bullets, {total} bytes -- against {whole} for the "
          f"whole file. Threshold: {floor}+ matching terms.")
    print("playbook = [")
    for n, b, _hit in scored:
        print(f"  {toml_str(b['selector'])},".ljust(66) + f"# {n} term(s), {b['bytes']} B")
    print("]")
    tail()
    print("\n# Paste over `[context] playbook`, then READ it and add to it. This ranks by what a")
    print("# bullet MENTIONS, which is a proxy for what a session needs and not the same thing.")
    print("# In particular the PROCESS traps -- how a session ends, how it reads, what a commit")
    print("# message may not carry, what to do when this pack is short -- name no crate and no")
    print("# file, so they score zero here and will never be proposed. They apply to every")
    print("# session regardless of file set; keep them in the list by hand.")
    print("# orient.py warns loudly if a selector stops matching, so a bullet that is later")
    print("# reworded fails loudly rather than silently.")
    return 0


def run_dupes(every: list[dict], floor: float) -> int:
    """Bullets that already say what another bullet says.

    An append-mostly file cannot notice that it already knows something. Five separate sessions
    wrote the `wsl.exe` path-mangling trap, one each, in five different wordings -- and every copy
    was charged to every session afterwards, because the playbook is the single largest thing
    `orient.py` ships. `--check` caught that pair only because two of the lead-ins happened to
    collide as selectors; three of the five it could not see at all.

    Similarity is over the *shingles* of each bullet -- its distinct three-word runs -- rather
    than over its characters, because the whole failure mode here is the same trap in different
    prose. A shared code span or path name is what the overlap actually rests on, so `--dupes`
    reports and never prunes: two bullets about the same file are often two different traps, and
    only a reader can tell.

    AGENTS.md is explicit that this file is append-mostly and must not be reworded to say the
    same thing differently. This is how you find the places where it already was."""
    def shingles(body):
        words = re.findall(r"[a-z0-9_./-]+", body.lower())
        return {" ".join(words[i:i + 3]) for i in range(max(0, len(words) - 2))}

    grams = [(b, shingles(b["body"])) for b in every]
    pairs = []
    for i, (a, ga) in enumerate(grams):
        for b, gb in grams[i + 1:]:
            if not ga or not gb:
                continue
            overlap = len(ga & gb) / min(len(ga), len(gb))
            if overlap >= floor:
                pairs.append((overlap, a, b))
    pairs.sort(key=lambda p: -p[0])

    kept: list[tuple[float, dict, dict]] = []
    deliberate: list[tuple[float, dict, dict, str]] = []
    for overlap, a, b in pairs:
        why = DELIBERATE_DISTINCT.get(tuple(sorted((a["selector"], b["selector"]))))
        if why:
            deliberate.append((overlap, a, b, why))
        else:
            kept.append((overlap, a, b))

    print(f"== BULLETS THAT MAY ALREADY BE SAID ELSEWHERE  (>= {floor:.0%} of the shorter one's "
          "three-word runs)")
    if not kept:
        print(f"  none at this threshold across {len(every)} bullets. "
              "`--dupes 0.15` lowers it -- `--min` is the `--match` term floor and does "
              "nothing here.")
    else:
        for overlap, a, b in kept:
            print(f"\n  {overlap:.0%}  and {a['bytes'] + b['bytes']:,} B between them")
            print(f"      {a['selector']}")
            print(f"      {b['selector']}")
        print(f"\n  {len(kept)} pair(s). This reports and never prunes -- two bullets about one "
              "file")
        print("  are often two different traps, and only a reader can tell. When they are the same")
        print("  trap, merge them into the better-written one and say so in the commit.")

    if deliberate:
        print("\n== PAIRS THAT OVERLAP ON PURPOSE  (already read; not a signal)")
        for overlap, a, b, why in deliberate:
            print(f"\n  {overlap:.0%}  {a['selector']}")
            print(f"       {b['selector']}")
            print(f"      -- {why}")
        print(f"\n  {len(deliberate)} pair(s), held in `DELIBERATE_DISTINCT` in this script. They")
        print("  are kept out of the list above so the run's duplicate signal can reach `none`;")
        print("  one bullet cites the other, so no merge can remove the overlap.")

    if floor <= DUPES_FLOOR:
        unseen = set(DELIBERATE_DISTINCT) - {tuple(sorted((a["selector"], b["selector"])))
                                             for _, a, b, _ in deliberate}
        if unseen:
            print("\n== DELIBERATE_DISTINCT ENTRIES THAT NO LONGER APPLY")
            for first, second in sorted(unseen):
                print(f"  {first}\n       {second}")
            print(f"\n  {len(unseen)} entry(s) matched no pair: either a bullet was reworded or")
            print("  deleted, or the overlap fell below the floor. Drop the entry from this script.")
    return 0


def run_check(text: str, every: list[dict]) -> int:
    print(f"docs/agent/playbook.md: {nbytes(text)} bytes, {len(every)} bullets\n")

    _expired, undeclared = report_expiry()

    print("\n== PATHS A BULLET NAMES THAT ARE NOT IN THE TREE")
    stale = 0
    splits = 0
    deliberate: list[tuple[str, str, str]] = []
    for b in every:
        gone = []
        for raw in re.findall(r"`([^`]+)`", b["body"]):
            cand = PATH_TRIM.sub("", raw.strip().split()[0] if raw.strip() else "")
            # `*` and `<` are the spellings of a path a bullet never claimed exists; an elision
            # -- `tests/conformance/io/…` -- is a third, and the bullet that spells one is often
            # the trap that the layout it names is the one the tree did NOT take.
            if not cand.startswith(TREE_DIRS) or any(m in cand for m in ("*", "<", "…", "...")):
                continue
            for one in brace_expand(cand):
                if (ROOT / one).exists():
                    continue
                why = DELIBERATE_STALE.get((b["selector"], one))
                if why is not None:
                    deliberate.append((b["selector"], one, why))
                else:
                    gone.append(one)
        if gone:
            stale += 1
            print(f"  {b['selector']}")
            for g in sorted(set(gone)):
                # `foo.rs` gone while `foo/` stands is a file that was SPLIT, not deleted -- the
                # module is still there and the trap is usually still live. Say so rather than
                # making every pass re-derive it; this annotates, it does not filter.
                asdir = Path(g).with_suffix("")
                if str(asdir) != g and (ROOT / asdir).is_dir():
                    splits += 1
                    print(f"      {g}  -- split into {asdir.as_posix()}/, so the module still stands")
                else:
                    print(f"      {g}")
    if not stale:
        # `loop.py`'s `gather_signals` reads this sentence to decide whether the stale-path signal fired,
        # so the first clause of it is a contract. What follows it is not.
        tail = f", or is quoted on purpose ({len(deliberate)} below)" if deliberate else ""
        print(f"  none -- every path any bullet names still exists{tail}")
    else:
        print(f"\n  {stale} bullet(s). A trap describing a file that is gone is usually a trap")
        print("  someone closed. Read it before deleting it; this reports, it never prunes.")
        if splits:
            print(f"  {splits} of the paths above are marked `split into` -- those are the weakest")
            print("  signal of the lot, because the code moved rather than went away.")

    if deliberate:
        print("\n== PATHS A BULLET QUOTES ON PURPOSE  (already read; not a signal)")
        for selector, path, why in deliberate:
            print(f"  {selector}")
            print(f"      {path}  -- {why}")
        print(f"\n  {len(deliberate)} bullet(s), held in `DELIBERATE_STALE` in this script. They are")
        print("  kept out of the list above so the run's stale-path signal can reach `none`; the trap")
        print("  each one describes IS its missing path, so no pass can ever prune them.")

    unseen = set(DELIBERATE_STALE) - {(s, p) for s, p, _ in deliberate}
    if unseen:
        print("\n== DELIBERATE_STALE ENTRIES THAT NO LONGER APPLY")
        for selector, path in sorted(unseen):
            print(f"  {selector}  ->  {path}")
        print(f"\n  {len(unseen)} entry(s) matched no bullet: either the bullet was reworded or")
        print("  deleted, or the path is back in the tree. Drop the entry from this script.")

    print("\n== SELECTORS THAT DO NOT RESOLVE TO EXACTLY ONE BULLET")
    bad = 0
    for b in every:
        hits, complaint = orientmod.slice_bullets(text, b["selector"])
        if complaint or len(hits) != 1:
            bad += 1
            print(f"  {b['selector']}  -> {complaint or f'{len(hits)} hits'}")
    if not bad:
        print(f"  none -- all {len(every)} bullets are individually selectable")

    report_growth(indent="  ")

    print("\n== WHAT EACH SECTION COSTS A SESSION THAT NAMES IT WHOLE")
    for head in sections(text):
        mine = [b for b in every if b["section"] == head]
        size = sum(b["bytes"] for b in mine)
        print(f"  {size:>6} B  {len(mine):>3} bullets   ## {head}")
    print()
    report_manifest(text, indent="  ")
    print("\n  Nothing here refuses over a size. This is a number to weigh when a goal is")
    print("  written, which is the only moment it can be acted on cheaply.")

    # A stale path and a size are judgement calls and stay reports. An unreachable selector is
    # not: `orient.py` fetches a bullet by exactly this string, so a goal naming one that
    # resolves to none or to two gets a trap it cannot be handed, and finds out never.
    if bad:
        print(f"\n  !! {bad} selector(s) above do not resolve to exactly one bullet. That is what")
        print("  this exits non-zero on: `orient.py` fetches a trap by its selector, so a goal's")
        print("  `[context] playbook` naming one of these is a trap the loop cannot deliver.")
        print("  Reword the colliding lead-in -- the bullet's text, not this tool, is the fix.")
    if undeclared:
        print(f"\n  !! {undeclared} bullet(s) declare nothing that retires them, or declare it in a")
        print("  form this tool cannot read. That is the other thing this exits non-zero on: a")
        print("  bullet without a trailer is one the file can never let go of. The syntax is in")
        print("  this script's module doc, and `session.py --wrap` refuses the same omission.")
    return 1 if (bad or undeclared) else 0


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--show", metavar="SELECTOR")
    ap.add_argument("--match", nargs="*", metavar="TERM")
    ap.add_argument("--manifest", nargs="*", metavar="TERM")
    ap.add_argument("--goal", action="store_true",
                    help="--manifest over loop-goal.toml's [context] modules")
    ap.add_argument("--min", type=int, default=2, metavar="N", dest="floor",
                    help="how many query terms a bullet must mention to be listed (default 2)")
    ap.add_argument("--gap", action="store_true",
                    help="bullets the handoff's next group implies that the manifest omits")
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--closes", metavar="SLUG",
                    help="exit 1 while a carried-gaps § Owned row names that goal")
    ap.add_argument("--retire", action="store_true",
                    help="delete every bullet whose declared retirement condition holds")
    ap.add_argument("--dry-run", action="store_true", help="with --retire: say what would go")
    # 0.22, not the 0.30 this shipped with: the `Core\\Math::gcd` twin trap was written down twice
    # at 23% overlap and the default was blind to it, while the whole 22-30% band held that one
    # pair and no false positive. This reports and never prunes, so the cost of looking lower is a
    # reader's minute.
    ap.add_argument("--dupes", nargs="?", type=float, const=DUPES_FLOOR, metavar="RATIO",
                    help="bullets that may already say what another bullet says (default 0.22)")
    opts = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    if not PLAYBOOK.exists():
        print(f"playbook.py: no {PLAYBOOK.relative_to(ROOT).as_posix()}")
        return 2

    text = read()
    every = all_bullets(text)

    if opts.show:
        return run_show(text, opts.show)
    if opts.gap:
        return run_gap(text, every, opts.floor)
    if opts.check:
        return run_check(text, every)
    if opts.closes:
        return run_closes(opts.closes)
    if opts.retire:
        return run_retire(opts.dry_run)
    if opts.dupes is not None:
        return run_dupes(every, opts.dupes)
    if opts.goal:
        terms = goal_terms()
        if not terms:
            print("playbook.py: loop-goal.toml names no `[context] modules`, so there is no "
                  "file set to match against. Pass terms to --manifest instead.")
            return 2
        return run_match(text, every, terms, as_manifest=True, floor=opts.floor)
    if opts.manifest is not None:
        return run_match(text, every, opts.manifest, as_manifest=True, floor=opts.floor)
    if opts.match is not None:
        return run_match(text, every, opts.match, as_manifest=False, floor=opts.floor)
    return run_index(text, every)


if __name__ == "__main__":
    sys.exit(main())
