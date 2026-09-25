#!/usr/bin/env python3
"""The loop's goal chain, edited by renaming files instead of by editing a list.

    python tools/chain.py                            # the order, with where the run stands
    python tools/chain.py --show 29                  # one goal, its files, its stages
    python tools/chain.py --new xml-stream --after 31 --title "a document walked once"
    python tools/chain.py --new xml-stream --before 31        # or --end
    python tools/chain.py --move 33 --to 22           # or --after 31, --before 31, --next
    python tools/chain.py --remove 33 --delete-files
    python tools/chain.py --retitle 31 --to unowned-sweep
    python tools/chain.py --set 31 --milestone M8
    python tools/chain.py --retire 29                # walked: drop the acceptance list it folded on
    python tools/chain.py --renumber                 # repair: close a hole, break a duplicate
    python tools/chain.py --check

**A goal's number is its position, and `docs/agent/goals/` is the whole schedule.** The numbers run
`1..N` with no gaps; sorting on them is walking the chain. So moving a goal *is* renaming its files,
and this tool is what makes that one operation instead of thirty.

A renumber is cheap for exactly one reason, and it is a rule rather than an accident: **prose names
a goal by its slug, never by its number** (AGENTS.md, *The schedule is the chain*). A slug does not
move; a number moves whenever anything is inserted in front of it. So the only text carrying a
number is the text this tool owns -- each goal's two file headers, and the link targets that are
filenames -- and a move rewrites those and nothing else. `--check` is the gate that keeps it true:
a `goal 29` written into prose is reported there, because the day one exists is the day a renumber
starts lying. `tools/goals.py` is the reader; this is the only writer.

Two rules are enforced rather than documented, because both fail silently:

* **The walked prefix is frozen.** `goal-switch.py` has folded each walked goal's checks into the
  one after it as its floor, and `.loop/chain.json` names the goal the run stands on. A goal at or
  before the live one that is moved, renumbered or removed invalidates a floor that is already
  built. Refused without `--force`, and so is a landing position inside that prefix.
* **A walked goal is retired, never removed.** `--retire N` deletes goal N's `.toml` and
  `.handoff.md` -- the `.md` stays, so the plan and the milestone files keep resolving -- and it is
  refused unless every `[[check]]` of that goal is *provably* in the live goal already. That is the
  whole safety argument for deleting an acceptance list, and it is a check rather than a note in a
  doc. Retirement is the `.toml` being gone; there is no flag to disagree with it.

Retirement exists because the fold is cumulative: goal `core-depth`'s checks are in goal `concurrency`'s file and in its
267-deep descendant, so a walked `.toml` is text that no tool reads and every `grep` over `docs/`
hits twice. The driver retires each goal as it leaves it.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import goals as goalsmod  # noqa: E402  -- the chain's one reader
import loop  # noqa: E402  -- same directory; the check schema has one home and it is `loop.py`
import orient as orientmod  # noqa: E402  -- the `[context]` manifest's one reader, likewise

ROOT = goalsmod.ROOT
GOALS = goalsmod.GOALS
README = GOALS / "README.md"

#: The goal the driver is actually running. `--retire` proves a walked goal's checks are in here
#: before deleting the file they came from; every switch since that goal left has folded them
#: forward one more time, so this is where all of them end up.
LIVE_GOAL = ROOT / "docs" / "agent" / "loop-goal.toml"

#: `goal-switch.py` inserts the previous goal's whole acceptance list at this line and refuses the
#: switch outright when it is missing -- which stops a run rather than degrading it. Every scaffold
#: this tool writes carries it, and `--check` is what says a hand-written goal forgot it.
MARKER = "# <<< goal-switch: floor checks are inserted below this line >>>"

SLUG_RE = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")

#: A goal named by its number in prose. Every one is a defect, because the number it names is a
#: position and moves. `--check` reports them; nothing here rewrites one, since a sentence built
#: around a number rarely survives having the number swapped out and a tool cannot tell which ones
#: do.
#:
#: The lookbehind is the one exception, and it is deliberate rather than incidental: a backtick
#: immediately in front means the text is a literal being *shown* -- the rule in AGENTS.md quotes
#: the wrong form to name it -- and quoting a spelling is not spelling it.
NUMBER_CITE_RE = re.compile(r"(?<![`\w])[Gg]oals?\s+\d+")

#: A goal's own header, found anywhere on a line rather than only where it opens one. The headers
#: below are the one place a number belongs, and a program that reads or writes goal files -- the
#: importer, its tests -- quotes them inside a string, which is the header being shown, not prose.
SHOWN_HEADER_RE = re.compile(r"#\s*Loop goal \d+|#\s*Goal \d+ --|\*\*Goal \d+ —")

#: The two headers that carry a goal's number, and the only text this tool rewrites besides the
#: filenames themselves. Both are the goal naming *itself*, which is the one place a number belongs.
H1_RE = re.compile(r"^(#\s*Loop goal )\d+", re.M)
TOML_HEAD_RE = re.compile(r"^(#\s*Goal )\d+( --)", re.M)
HANDOFF_RE = re.compile(r"^(\*\*Goal )\d+( —)", re.M)

#: `29-xml-tree` inside a link or a path. Rewritten from the stem map alone, so a string that merely
#: looks like one -- a date, a version -- is left exactly as it was.
STEM_RE = re.compile(r"\b\d+-[a-z0-9]+(?:-[a-z0-9]+)*\b")


def die(message):
    print(f"chain: {message}", file=sys.stderr)
    return 2


def rel(path):
    return goalsmod.rel(path)


def git(*args, check=True):
    r = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    if check and r.returncode != 0:
        raise SystemExit(die(f"git {' '.join(args)} failed -- {r.stderr.strip()}"))
    return r.stdout


def find(chain, num):
    g = goalsmod.find(chain, num)
    if g is None:
        raise SystemExit(die(f"no goal {num} on the chain -- `python tools/chain.py` lists it"))
    return g


def generated(goal):
    """Whether this goal sits in a subdirectory of `docs/agent/goals/`, where no hand-written goal is.

    The test is the directory, and not a marker in the goal's prose: reading it off the word
    "generated" in the opening paragraph found a hand-written goal whose subject happened to be
    generated files. `bun nv proofs` writes its goals beside the others with a slug name, so none of
    them is in a subdirectory and this is false for every goal on the chain.
    """
    return goal.folder != GOALS


# ---------------------------------------------------------------------------------------------------
# Renumbering, which is the whole tool
# ---------------------------------------------------------------------------------------------------


def repo_files():
    """Every text file git would show you -- tracked, plus untracked and not ignored.

    The untracked half is the one that matters. A goal is three files this tool writes and nobody
    has committed yet, so a set built from `git ls-files` alone is blind to exactly the file whose
    TODOs are still being filled in -- which is the only moment a citation gate is worth having.
    """
    skip = {".png", ".jpg", ".jpeg", ".ico", ".svg", ".lock", ".woff", ".woff2"}
    for line in git("ls-files", "--cached", "--others", "--exclude-standard").split("\n"):
        line = line.strip()
        if not line:
            continue
        path = ROOT / line
        if path.is_file() and path.suffix.lower() not in skip:
            yield path


def rewrite_stems(text, stems):
    """`26-xml-tree` -> `29-xml-tree`, in a link target or a path and nowhere else."""
    return STEM_RE.sub(lambda m: stems.get(m.group(0), m.group(0)), text)


def rewrite_headers(goal, num):
    """Renumber the headers in which a goal names itself. Nothing else in the file is touched."""
    for path, pattern, repl in ((goal.md, H1_RE, rf"\g<1>{num}"),
                                (goal.toml, TOML_HEAD_RE, rf"\g<1>{num}\g<2>"),
                                (goal.handoff, HANDOFF_RE, rf"\g<1>{num}\g<2>")):
        if path.is_file():
            text = path.read_text(encoding="utf-8")
            out = pattern.sub(repl, text, count=1)
            if out != text:
                path.write_text(out, encoding="utf-8", newline="\n")


def number_citations():
    """Every `goal 29` written into prose, as `(path, line number, text)`.

    A finding, never a fix. AGENTS.md's *The schedule is the chain* is the rule -- a goal is named
    by its slug, because a number is a position and a position moves -- and this is what makes the
    rule a gate instead of a hope. `docs/agent/goals/README.md`'s own table is the one exception:
    it is a listing of the chain in order, so the number IS what it is showing.
    """
    out = []
    for path in repo_files():
        if path == README:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        for i, line in enumerate(text.split("\n"), 1):
            if H1_RE.match(line) or TOML_HEAD_RE.match(line) or HANDOFF_RE.match(line):
                continue
            shown = [h.span() for h in SHOWN_HEADER_RE.finditer(line)]
            for m in NUMBER_CITE_RE.finditer(line):
                if not any(a <= m.start() < b for a, b in shown):
                    out.append((rel(path), i, m.group(0)))
    return out


def apply_renumber(chain, mapping, dry_run):
    """Rename every goal the map moves, and fix up the text that names a number.

    `mapping` is `{old number: new number}`, and the callers below build it by rebuilding the whole
    order rather than by reasoning about which goals in between shift -- so it is always a
    permutation, and a move never has to know which direction it went.

    Three kinds of text carry a number and all three are this tool's: the filenames, each goal's
    own headers, and a link target that is a filename. Prose carries none, by the rule in this
    module's docstring, which is why this is a rename and not a rewrite.

    Returns `(renamed, rewritten, stale)`: files moved, the paths whose link targets changed, and
    every `goal 29` found in prose -- a rule violation this reports and does not touch.
    """
    moves = {old: new for old, new in mapping.items() if old != new}
    by_num = {g.num: g for g in chain}
    if not moves:
        return 0, [], []
    stems = {by_num[old].stem: f"{new}-{by_num[old].slug}" for old, new in moves.items()}

    if dry_run:
        for old, new in sorted(moves.items()):
            print(f"       goal {old} -> {new}  {by_num[old].slug}")
        return 0, [], number_citations()

    # The rename goes through a temporary name because the map is a permutation: 21 -> 7 and 7 -> 9
    # both want the same directory, and either order overwrites one of them going straight across.
    # A rename keeps each goal's files in the folder they are already in, and the number is the
    # only thing moving: the folder is what `generated` reads.
    pairs = []
    for old, new in moves.items():
        g = by_num[old]
        folder = rel(g.folder)
        for suffix in (".md", ".toml", ".handoff.md"):
            if (g.folder / f"{g.stem}{suffix}").is_file():
                pairs.append((f"{folder}/{g.stem}{suffix}",
                              f"{folder}/__renumber__{new}-{g.slug}{suffix}",
                              f"{folder}/{new}-{g.slug}{suffix}"))
    for src, via, _ in pairs:
        git("mv", src, via)
    for _, via, dst in pairs:
        git("mv", via, dst)

    for old, new in moves.items():
        rewrite_headers(goalsmod.Goal(new, by_num[old].slug, by_num[old].folder), new)

    rewritten = []
    # The live copies are copies of the live goal and hold the same link targets, so they are
    # rewritten beside the tracked tree rather than after it.
    live_copies = [ROOT / "docs" / "agent" / n
                   for n in ("loop-goal.md", "loop-goal.toml", "handoff.md")]
    for path in list(repo_files()) + [p for p in live_copies if p.is_file()]:
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        out = rewrite_stems(text, stems)
        if out != text:
            path.write_text(out, encoding="utf-8", newline="\n")
            rewritten.append(rel(path))

    live = goalsmod.live()
    if live in moves:
        goalsmod.write_live(moves[live])
    return len(pairs), rewritten, number_citations()


def report(renamed, rewritten, stale):
    print(f"       {renamed} file(s) renamed, {len(rewritten)} file(s) had a link target rewritten")
    # Naming them is the whole point. The renames are in the index already -- `git mv` put them
    # there -- and these rewrites are ordinary writes, so a commit of what `git status` shows as
    # staged lands half of one operation: the file is at its new name and a link somewhere else
    # still points at the old one, which is a broken link at HEAD that only CI finds.
    if rewritten:
        print("       Not staged, while the renames are. Commit them with the goal's own files:")
        for path in rewritten[:20]:
            print(f"         {path}")
        if len(rewritten) > 20:
            print(f"         ... and {len(rewritten) - 20} more")
    if stale:
        print()
        print(f"       {len(stale)} place(s) name a goal by NUMBER, which this did not touch and")
        print("       which a renumber has just made wrong. A goal is named by its slug:")
        for path, line, text in stale[:20]:
            print(f"         {path}:{line}  {text}")
        if len(stale) > 20:
            print(f"         ... and {len(stale) - 20} more")
    print("       `python tools/plan.py --sync` -- `Carried by` cells hold the numbers.")


def order_after(order):
    """`{old: new}` for a chain rewritten into `order`, the goals in their new sequence."""
    return {g.num: i for i, g in enumerate(order, 1)}


def frozen(num, force, what):
    """Refuse to touch a goal the run has already walked. True when it is refused."""
    live = goalsmod.live()
    if force or not live or num > live:
        return False
    where = "is the live goal" if num == live else "has already been walked"
    die(f"goal {num} {where} -- {what} it invalidates the floor goal-switch.py already folded into "
        f"the goals after it.\n"
        f"       {rel(goalsmod.STATE)} says the run stands on goal {live}. Pass --force if the run "
        f"is over or was never started.")
    return True


def landing(num, force, what):
    """Refuse a landing position at or before the live goal. True when it is refused."""
    live = goalsmod.live()
    if force or not live or num > live:
        return False
    die(f"goal {num} is at or before the live goal ({live}) -- {what} there renumbers goals the run "
        f"has already walked. Pass --force if the run is over or was never started.")
    return True


# ---------------------------------------------------------------------------------------------------
# Reading the chain out
# ---------------------------------------------------------------------------------------------------


def cmd_list(chain, show_all):
    live = goalsmod.live()
    hand = [g for g in chain if not generated(g)]
    shown = chain if show_all else hand
    print(f"chain: {rel(GOALS)} -- {len(chain)} goal(s), {len(chain) - len(hand)} of them generated"
          + ("" if show_all or len(hand) == len(chain) else " (--all to list those too)"))
    if live:
        print(f"       {rel(goalsmod.STATE)}: the run stands on goal {live}")
    else:
        print(f"       {rel(goalsmod.STATE)}: no run has installed a goal yet")
    fail = goalsmod.numbering_error(chain)
    if fail:
        print(f"       !! {fail}")
    print()
    # The number is its own column rather than a prefix on the name. This listing is the one place
    # a goal's position is the subject, and joining the two is what taught every other line in the
    # driver to print `29 xml-tree` as though that were what the goal is called.
    print(f"  {'#':>3}  {'goal':<24} {'milestone':<12} {'preflight':<9} state")
    for g in chain:
        if g not in shown:
            continue
        state = "walked" if g.num < live else "LIVE" if g.num == live else "ahead"
        if g.retired:
            state += ", retired"
        missing = [p.name for p in g.files if not p.is_file()]
        if missing:
            state += f"  !! {', '.join(missing)} missing"
        print(f"  {g.num:>3}  {g.slug:<24} {g.milestone:<12} {g.preflight:<9} {state}")
    print()
    print("  A goal at or before the live one is frozen: its checks are already somebody's floor.")
    print("  A retired one has had that list dropped -- it is in the live goal, not in its own file.")
    return 0


def cmd_show(chain, num):
    g = find(chain, num)
    live = goalsmod.live()
    print(f"chain: goal `{g.slug}`, {g.num} of {len(chain)} -- "
          + ("walked" if g.num < live else "LIVE" if g.num == live else "ahead of the run")
          + (", retired (its checks are the live goal's floor)" if g.retired else ""))
    print(f"       {g.title}")
    print(f"       milestone {g.milestone or '(none)'}"
          + (f", preflight {g.preflight}" if g.preflight else ""))
    print()
    for path in (g.md, g.toml, g.handoff):
        mark = " " if path.is_file() else ("-" if g.retired else "!")
        print(f"  {mark} {rel(path)}")
    if g.toml.is_file():
        text = g.toml.read_text(encoding="utf-8")
        stages = []
        for m in re.finditer(r'^\s*stage\s*=\s*"([^"]*)"', text, re.M):
            if m.group(1) not in stages:
                stages.append(m.group(1))
        print()
        print(f"  stages   {', '.join(stages) if stages else '(no [[check]] yet)'}")
        print("  floor    " + ("marker present" if MARKER in text else
                               "NO MARKER -- goal-switch.py will refuse this goal"))
    why = re.search(r"^## Why here\n\n(.*?)(?=\n## |\Z)", g.md.read_text(encoding="utf-8"),
                    re.S | re.M)
    if why:
        print()
        print("  why here")
        for line in why.group(1).strip().split("\n"):
            print(f"    {line}")
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
        # inherit silently. The table is also what `goals.py` reads the preflight off, so it is not
        # decoration: a goal with no `[docker]` table is a goal the driver will not preflight.
        keys = "\n".join(ln for ln in got.split("\n") if not ln.lstrip().startswith("#")) if got \
            else '[docker]\ncompose = "tests/db/compose.yaml"\nservices = []\nmemoize_on = []'
        tables.append("# TODO: why this goal needs a daemon -- its own checks, its floor's, or\n"
                      "# both. The driver preflights it before this goal's first session, and this\n"
                      "# table being here is what says so.\n" + keys.strip("\n"))
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

# TODO: the two or three RULE IDS -- `topic/slug`, not an ADR number -- that every stage of this
# goal works inside. A rule id prints the fragment, which is the rule; a record number prints its
# rules' titles, which is a pointer to them. Name a record here only for the surrounding map, and
# put the rules a single stage is written against in that stage's own table below.
rules = []

# TODO: usually empty. An ADR section argues one stage's design, so it belongs in that stage's
# table below -- naming it here prints it to every session of every stage.
adrs = []

# TODO: the conventions.md shapes this goal writes -- the ones EVERY stage writes.
shapes = ["A commit message"]

# TODO: the milestone this goal builds inside, as "M8:verify" for its acceptance paragraph alone.
milestones = []

{carried[0]}

{carried[1]}


# ---------------------------------------------------------------------------------------------------
# THE STAGES. One table per prose stage in `{num}-{slug}.md`, holding what only that stage needs.
#
# `orient.py` applies the table whose number `handoff.md`'s `## Next group` names and appends it to
# the base above -- an overlay adds and never replaces, so the base is what a session needs whatever
# stage it is on, and the way it gets smaller is by moving an entry down here. A stage with nothing
# of its own needs no table. Narrowable: rules, adrs, spec, shapes, playbook, milestones.
#
# These are the PROSE stage numbers, the `## Stage N` headings in this goal's `.md`. They are not
# the `[[check]]` blocks' `stage = "2 TODO"` labels, which may group several prose stages under one
# acceptance line. loop-authoring.md section 2 is the whole of it.
# ---------------------------------------------------------------------------------------------------

[context.stage.2]
# TODO: the rule ids stage 2 is written against, and the ADR sections that argue it.
rules = []
adrs = []

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


def scaffold_md(num, slug, title, milestone, prev_slug):
    """The goal's prose, as TODOs for whoever asked for the goal.

    Every goal it names, it names by **slug**. This file is one an insert in front of it renumbers,
    so a number written into a sentence here would come to name a goal that is not the one meant --
    and the scaffold is where that habit would be taught. The `# Loop goal N` header is the goal
    naming itself, which is the one place a number belongs.
    """
    floor = (f"Goal `{prev_slug}`'s whole acceptance list" if prev_slug
             else "The live goal's whole acceptance list")
    return f"""---
milestone: {milestone}
---
# Loop goal {num} — {title}

TODO: the target, in two or three sentences — what is different about the language, the runtime or
the tooling once this goal is green. Not the work; the outcome.

## Why here

TODO: why this goal sits here rather than anywhere else on the chain. The order is a dependency
chain, not a preference, and this section is the only home of the reason. Say what it needs that is
already built, and what after it needs this — not what it does, which is above.

## Stage 0 — the catch-up

TODO: the sentences already on disk that this goal makes wrong, each with the file that holds them.
Delete this stage if there are none.

## Stage 1 — the floor

{floor}, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — TODO, the keystone

TODO: the one thing that, once it exists, makes every stage after it mechanical.

## Standing decisions

TODO: the calls a session will meet and must not stop to ask about. loop-authoring.md § 5 is the
shape; a goal without this section is a goal that eventually holds the run on `BLOCKED`.
"""


def scaffold_handoff(num, title, prev_slug, next_slug):
    """The seed handoff. It names its neighbours by slug, for `scaffold_md`'s reason."""
    prev = (f"Goal `{prev_slug}`'s whole list is this goal's Stage 1 floor." if prev_slug else
            "The previous goal's whole list is this goal's Stage 1 floor.")
    tail = (f"- When this goal's last check goes green the driver takes goal `{next_slug}`."
            if next_slug else
            "- When this goal's last check goes green the driver takes the next goal on the chain.")
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


# ---------------------------------------------------------------------------------------------------
# Editing the order
# ---------------------------------------------------------------------------------------------------


def target_position(chain, opts, moving):
    """The number a `--new`/`--move` lands on, from `--after`/`--before`/`--next`/`--to`/`--end`.

    `moving` says whether the goal is already on the chain: an insert may land one past the end, a
    move may not, because the goal it would go after is itself.
    """
    if opts.end:
        # In front of a goal that says `position: last`, which is what "the end" means while one
        # exists; moving such a goal is the one thing that lands behind another.
        behind = [g for g in goalsmod.pinned_tail(chain) if g.num != moving]
        if moving and goalsmod.find(chain, moving).pinned_last:
            behind = []
        return (len(chain) if moving else len(chain) + 1) - len(behind)
    if opts.next:
        live = goalsmod.live()
        if not live:
            raise SystemExit(die("--next means \"directly after the live goal\", and "
                                 f"{rel(goalsmod.STATE)} records none"))
        return live + 1
    if opts.after is not None:
        return find(chain, opts.after).num + 1
    if opts.before is not None:
        return find(chain, opts.before).num
    if opts.to is not None:
        try:
            return int(opts.to)
        except ValueError:
            raise SystemExit(die(f"--to takes a goal number, not {opts.to!r}"))
    return None


def cmd_new(chain, opts):
    slug = opts.new.strip().strip("/")
    if not SLUG_RE.fullmatch(slug):
        return die(f"{slug!r} is not a goal slug -- lowercase words joined by hyphens, as in "
                   f"`unix-sockets`; it becomes the filenames and the goal's name")
    if any(g.slug == slug for g in chain):
        return die(f"a goal named {slug!r} already exists")

    at = target_position(chain, opts, moving=0)
    if at is None:
        return die("say where it goes: --after N, --before N, --next, --end, or --to N")
    if at < 1 or at > len(chain) + 1:
        return die(f"goal {at} is outside 1..{len(chain) + 1}")
    if landing(at, opts.force, "inserting"):
        return 2

    prev = goalsmod.find(chain, at - 1)
    nxt = goalsmod.find(chain, at)
    title = opts.title or f"TODO ({slug})"
    milestone = opts.milestone or (prev.milestone if prev else "post-parity")
    docker = opts.docker if opts.docker is not None else bool(prev and prev.preflight)

    # Everything from the landing position on shifts up one, so the new number is free.
    order = [g for g in chain if g.num < at] + [g for g in chain if g.num >= at]
    mapping = {g.num: (i if g.num < at else i + 1) for i, g in enumerate(order, 1)}

    if opts.dry_run:
        print(f"chain: --dry-run -- goal {at} {slug} would be inserted, "
              f"{sum(1 for o, n in mapping.items() if o != n)} goal(s) shifting up")
        apply_renumber(chain, mapping, dry_run=True)
        return 0

    renamed, rewritten, ranges = apply_renumber(chain, mapping, dry_run=False)

    stem = f"{at}-{slug}"
    files = {
        GOALS / f"{stem}.md": scaffold_md(at, slug, title, milestone, prev.slug if prev else None),
        GOALS / f"{stem}.toml": scaffold_toml(at, slug, title,
                                              prev.toml if prev and not prev.retired else None,
                                              docker),
        GOALS / f"{stem}.handoff.md": scaffold_handoff(at, title, prev.slug if prev else None,
                                                       nxt.slug if nxt else None),
    }
    existing = [rel(p) for p in files if p.exists()]
    if existing:
        return die(f"refusing to overwrite: {', '.join(existing)}")
    for path, body in files.items():
        path.write_text(body, encoding="utf-8", newline="\n")

    print(f"chain: goal {at} {slug} inserted; the chain is {len(chain) + 1} goal(s)")
    for path in files:
        print(f"       wrote {rel(path)}")
    report(renamed, rewritten, ranges)
    print()
    print("Next, in this order:")
    print(f"  1. Fill the TODOs in {rel(GOALS / (stem + '.md'))} -- the target, `## Why here`, the")
    print("     stages, the standing decisions. loop-authoring.md is how.")
    print(f"  2. Fill {rel(GOALS / (stem + '.toml'))}: the `[context]` manifest first (it is the")
    print("     session's whole read budget), then one `[[check]]` per stage. Leave the marker.")
    print(f"  3. Add its row to {rel(README)}.")
    print("  4. `python tools/plan.py --sync`, then `python tools/chain.py --check`.")
    return 0


def cmd_move(chain, opts):
    g = find(chain, opts.move)
    at = target_position(chain, opts, moving=g.num)
    if at is None:
        return die("say where it goes: --to N, --after N, --before N, --next, or --end")
    if at < 1 or at > len(chain):
        return die(f"goal {at} is outside 1..{len(chain)}")
    if frozen(g.num, opts.force, "moving") or landing(min(at, g.num), opts.force, "landing"):
        return 2
    if at == g.num:
        return die(f"goal {g.num} is already there")

    rest = [x for x in chain if x.num != g.num]
    mapping = order_after(rest[:at - 1] + [g] + rest[at - 1:])
    if opts.dry_run:
        print(f"chain: --dry-run -- goal `{g.slug}` ({g.num}) would become goal {at}")
        apply_renumber(chain, mapping, dry_run=True)
        return 0
    renamed, rewritten, ranges = apply_renumber(chain, mapping, dry_run=False)
    print(f"chain: goal `{g.slug}` ({g.num}) is now goal {at}")
    report(renamed, rewritten, ranges)
    return 0


def cmd_remove(chain, opts):
    g = find(chain, opts.remove)
    if frozen(g.num, opts.force, "removing"):
        return 2
    on_disk = [p for p in (g.md, g.toml, g.handoff) if p.is_file()]
    if not opts.delete_files:
        return die(f"goal `{g.slug}` ({g.num}) owns {len(on_disk)} file(s). Removing it deletes them "
                   f"and closes the hole its number leaves:\n"
                   + "".join(f"       {rel(p)}\n" for p in on_disk)
                   + "       Pass --delete-files to mean it. A goal the run has WALKED is retired "
                     "instead (--retire), which keeps its prose.")
    if opts.dry_run:
        print(f"chain: --dry-run -- would delete {len(on_disk)} file(s) and close the hole at "
              f"goal {g.num}")
        return 0
    # `--ignore-unmatch` because a goal scaffolded and not yet staged is untracked, and `git rm`
    # refuses a path it has never seen -- which is exactly the goal most likely to be removed
    # again. The unlink after it is what actually deletes those.
    git("rm", "-q", "--ignore-unmatch", "--", *(rel(p) for p in on_disk))
    for path in on_disk:
        path.unlink(missing_ok=True)
    rest = [x for x in chain if x.num != g.num]
    renamed, rewritten, ranges = apply_renumber(rest, order_after(rest), dry_run=False)
    print(f"chain: goal `{g.slug}` ({g.num}) removed, {len(on_disk)} file(s) deleted, hole closed")
    report(renamed, rewritten, ranges)
    print(f"       Prose naming `{g.slug}` now names a goal that is gone -- `git grep -n "
          f"'{g.slug}'` is that list, and this tool does not write sentences.")
    return 0


def cmd_renumber(chain, opts):
    """Repair: force `1..N` over whatever is on disk, in the order the numbers already imply."""
    mapping = order_after(sorted(chain, key=lambda g: g.num))
    if not any(o != n for o, n in mapping.items()):
        print(f"chain: the {len(chain)} goal(s) are already numbered 1..{len(chain)}")
        return 0
    renamed, rewritten, ranges = apply_renumber(chain, mapping, dry_run=opts.dry_run)
    if not opts.dry_run:
        print(f"chain: renumbered to 1..{len(chain)}")
        report(renamed, rewritten, ranges)
    return 0


def cmd_retitle(chain, opts):
    g = find(chain, opts.retitle)
    if frozen(g.num, opts.force, "renaming"):
        return 2
    slug = opts.to.strip()
    if not SLUG_RE.fullmatch(slug):
        return die(f"{slug!r} is not a goal slug -- lowercase words joined by hyphens")
    if slug == g.slug:
        return die("that is the slug it already has")
    old_stem, new_stem = g.stem, f"{g.num}-{slug}"
    if opts.dry_run:
        print(f"chain: --dry-run -- would rename {old_stem} -> {new_stem}")
        return 0
    for suffix in (".md", ".toml", ".handoff.md"):
        if (GOALS / f"{old_stem}{suffix}").is_file():
            git("mv", f"docs/agent/goals/{old_stem}{suffix}",
                f"docs/agent/goals/{new_stem}{suffix}")
    rewritten = []
    for path in repo_files():
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        out = text.replace(old_stem, new_stem)
        if out != text:
            path.write_text(out, encoding="utf-8", newline="\n")
            rewritten.append(rel(path))
    print(f"chain: {old_stem} -> {new_stem}; {len(rewritten)} file(s) had the stem rewritten")
    print("       The number did not move, so no citation changed meaning.")
    if rewritten:
        print("       Not staged, while the renames are. Commit them with the goal's own files:")
        for path in rewritten[:20]:
            print(f"         {path}")
        if len(rewritten) > 20:
            print(f"         ... and {len(rewritten) - 20} more")
    return 0


def cmd_set(chain, opts):
    g = find(chain, opts.set)
    if not opts.milestone:
        return die("--set takes --milestone; everything else a goal carries is in its own files")
    text = g.md.read_text(encoding="utf-8")
    m = goalsmod.FRONT_RE.match(text)
    block = f"---\nmilestone: {opts.milestone}\n---\n"
    if opts.dry_run:
        print(f"chain: --dry-run -- goal {g.num} milestone {g.milestone!r} -> {opts.milestone!r}")
        return 0
    g.md.write_text(block + (text[m.end():] if m else text), encoding="utf-8", newline="\n")
    print(f"chain: goal `{g.slug}` ({g.num}) is milestone {opts.milestone}")
    print("       `python tools/plan.py --sync` -- the `Carried by` cell is derived from this.")
    return 0


# ---------------------------------------------------------------------------------------------------
# Retiring a walked goal
# ---------------------------------------------------------------------------------------------------


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


def cmd_retire(chain, opts):
    """Drop a walked goal's acceptance list, keeping its prose.

    The deletion is safe for one reason and it is checked rather than asserted: `goal-switch.py`
    folded every `[[check]]` of this goal into the next one at the switch, and each switch since
    folded that forward again, so the live goal holds them all. This proves that before unlinking
    anything, and the proof is not `--force`-able -- a floor that has gone missing is the one thing
    retirement must never hide, and `--force` is for a run whose position bookkeeping is stale.
    """
    g = find(chain, opts.retire)
    live = goalsmod.live()
    if g.retired:
        return die(f"goal `{g.slug}` ({g.num}) is already retired")
    if g.num >= live and not opts.force:
        where = ("is the live goal -- its `.toml` is the file the driver runs" if g.num == live else
                 "is ahead of the run" if live else
                 f"cannot be retired: {rel(goalsmod.STATE)} says no run has installed a goal")
        return die(f"goal `{g.slug}` ({g.num}) {where}. Only a goal the run has LEFT may be retired, "
                   f"because leaving it is what folded its checks forward.\n"
                   f"       Pass --force if the run is over and this position is stale.")
    if not LIVE_GOAL.is_file():
        return die(f"{rel(LIVE_GOAL)} does not exist, so there is nothing to prove the fold into")
    lost = sorted(check_ids(g.toml) - check_ids(LIVE_GOAL))
    if lost:
        die(f"goal `{g.slug}` ({g.num}) holds {len(lost)} check(s) that {rel(LIVE_GOAL)} does not, "
            f"so its acceptance list was NOT folded all the way forward:")
        for kind, name in lost[:12]:
            print(f"       [{kind}] {name}", file=sys.stderr)
        if len(lost) > 12:
            print(f"       ... and {len(lost) - 12} more", file=sys.stderr)
        print("       Retiring it would delete a floor nothing else carries. Refusing.",
              file=sys.stderr)
        return 2

    victims = [g.toml, g.handoff]

    # The other way this deletion goes wrong, and the one the fold proof says nothing about: a
    # markdown link to a file that is about to stop existing. `check-links.py` is a CI gate and
    # `session.py --wrap` runs it in-process, so a switch that committed one would refuse the NEXT
    # session's wrap -- unattended, hours later, in a file that session never touched.
    names = {p.name for p in victims}
    cites = []
    for doc in sorted((ROOT / "docs").rglob("*.md")):
        for target in re.findall(r"\]\(([^)]+)\)", doc.read_text(encoding="utf-8")):
            if Path(target.split("#", 1)[0]).name in names:
                cites.append(f"{rel(doc)} -> {target}")
    if cites:
        die(f"goal `{g.slug}` ({g.num}) cannot be retired yet: {len(cites)} link(s) name a file it "
            f"would delete, and check-links.py is a gate:")
        for line in cites[:8]:
            print(f"       {line}", file=sys.stderr)
        print("       Rewrite those sentences first -- a retired goal's checks are the live "
              "goal's floor, and that is what they should say.", file=sys.stderr)
        return 2

    freed = sum(p.stat().st_size for p in victims if p.is_file())
    if opts.dry_run:
        print(f"chain: --dry-run -- would retire goal `{g.slug}` ({g.num}): every one of its "
              f"{len(check_ids(g.toml))} distinct checks is in {rel(LIVE_GOAL)}, so "
              f"{', '.join(rel(p) for p in victims)} ({freed // 1024}K) would go")
        return 0
    for path in victims:
        if path.is_file():
            path.unlink()
    print(f"chain: goal `{g.slug}` ({g.num}) retired -- its whole acceptance list is in "
          f"{rel(LIVE_GOAL)}, and its `.toml` being gone is what records that")
    print(f"       {freed // 1024}K freed. {rel(g.md)} stays: {rel(README)}, the plan and the "
          f"milestone files cite it.")
    return 0


# ---------------------------------------------------------------------------------------------------
# The gate
# ---------------------------------------------------------------------------------------------------


def cmd_check(chain):
    problems, notes = [], []
    live = goalsmod.live()

    fail = goalsmod.numbering_error(chain)
    if fail:
        problems.append(fail)
    if not chain:
        problems.append(f"{rel(GOALS)} holds no goal -- the driver has nothing to walk")

    for g in chain:
        where = f"goal `{g.slug}` ({g.num})"
        for path in g.files:
            if not path.is_file():
                problems.append(f"{where}: {rel(path)} does not exist")
        if not g.md.is_file():
            continue

        text = g.md.read_text(encoding="utf-8")
        if not goalsmod.FRONT_RE.match(text):
            problems.append(f"{rel(g.md)}: no front matter -- the milestone has nowhere to live")
        if not g.milestone:
            problems.append(f"{where}: its `.md` front matter names no `milestone` -- `plan.py "
                            f"--check` derives every `Carried by` cell from that key")
        h1 = goalsmod.FRONT_RE.sub("", text, count=1).split("\n", 1)[0]
        if not h1.startswith(f"# Loop goal {g.num} "):
            notes.append(f"{rel(g.md)}: its H1 is {h1[:60]!r}, which `plan.py`'s `live_goal()` "
                         f"matches the live copy against")
        if g.num > live and "\n## Why here" not in text and not generated(g):
            notes.append(f"{rel(g.md)}: no `## Why here` section. The order is a dependency chain "
                         f"and that section is the only home of the reason this goal sits at "
                         f"{g.num}")
        # loop-authoring.md § 5: anything a session could reasonably stop and ask about eventually
        # holds the run on `BLOCKED`, and this section is the only place a goal says a call is
        # already made.
        if g.num > live and "\n## Standing decisions" not in text:
            notes.append(f"{rel(g.md)}: no `## Standing decisions` section -- loop-authoring.md "
                         f"§ 5 is where a goal pre-authorizes the calls its stages reach")

        if g.retired:
            if g.handoff.is_file():
                notes.append(f"{where}: retired, but {g.handoff.name} is still on disk -- "
                             f"`--retire` deletes it and something put it back")
            # `live` is 0 in a tree with no `.loop/chain.json` at all, which is every fresh clone
            # and every CI job: a retired goal there is history, not a contradiction.
            if live and g.num >= live:
                problems.append(f"{where}: is retired but the run stands on goal {live} -- only a "
                                f"goal the run has LEFT may be retired, since retiring is what "
                                f"says its checks are already somebody's floor")
            continue

        body = g.toml.read_text(encoding="utf-8")
        if MARKER not in body:
            problems.append(f"{rel(g.toml)}: no goal-switch marker line, so the switch into this "
                            f"goal refuses and the run stops. Add:\n      {MARKER}")
        for need, why in (("files", "goal-switch.py unions the previous goal's fixtures into it "
                                    "and refuses when the key is absent"),
                          ("skip", "`[valgrind] skip` is unioned the same way")):
            if not re.search(rf"^\s*{need}\s*=\s*\[", body, re.M):
                problems.append(f"{rel(g.toml)}: no `{need} = [...]` -- {why}")
        if "[[check]]" not in body:
            problems.append(f"{rel(g.toml)}: holds no `[[check]]`, so nothing can turn it green "
                            f"and the run stalls on it")
        # The list has to be one the DRIVER can run, not just one that greps right. Everything
        # above is a text search; this is `loop.py`'s own schema, so a misspelled key is a problem
        # printed here rather than a dead run on the night the chain reaches the goal.
        fail = loop.spec_error(g.toml)
        if fail:
            problems.append(f"{rel(g.toml)}: the driver cannot run this list -- {fail}")
        # The other half of walkable: the driver can run the checks, and the SESSION gets the pack.
        # A manifest naming a heading that is not there costs nothing until the chain reaches the
        # goal, and then costs one session the section it needed.
        bad, said = orientmod.manifest_findings(g.toml)
        problems.extend(bad)
        notes.extend(said)
        if not g.handoff.read_text(encoding="utf-8").startswith("# Handoff"):
            notes.append(f"{rel(g.handoff)}: does not start with `# Handoff`")
        for path in g.files:
            if "TODO" in path.read_text(encoding="utf-8"):
                notes.append(f"{rel(path)}: still carries TODO markers -- a scaffold nobody has "
                             f"filled in yet")

    # Side goals: never walked, so no number and no floor of their own on disk, but the same
    # three files and a list `loop.py --side` can run (goals/README.md § *Side goals*).
    problems.extend(goalsmod.side_errors())
    for g in goalsmod.load_side():
        if g.retired:
            continue
        h1 = goalsmod.FRONT_RE.sub("", g.md.read_text(encoding="utf-8"),
                                   count=1).lstrip("\n").split("\n", 1)[0]
        if not h1.startswith("# Side goal "):
            problems.append(f"{rel(g.md)}: its H1 is {h1[:60]!r}; a side goal's opens "
                            f"`# Side goal — `")
        body = g.toml.read_text(encoding="utf-8")
        if MARKER not in body:
            problems.append(f"{rel(g.toml)}: no goal-switch marker line, so main's floor has "
                            f"nowhere to go. Add:\n      {MARKER}")
        for need in ("files", "skip"):
            if not re.search(rf"^\s*{need}\s*=\s*\[", body, re.M):
                problems.append(f"{rel(g.toml)}: no `{need} = [...]` -- main's floor is unioned "
                                f"into it")
        if "[[check]]" not in body:
            problems.append(f"{rel(g.toml)}: holds no `[[check]]`, so nothing can turn it green")
        fail = loop.spec_error(g.toml)
        if fail:
            problems.append(f"{rel(g.toml)}: the driver cannot run this list -- {fail}")
        bad, said = orientmod.manifest_findings(g.toml)
        problems.extend(bad)
        notes.extend(said)
        if not g.handoff.read_text(encoding="utf-8").startswith("# Handoff"):
            notes.append(f"{rel(g.handoff)}: does not start with `# Handoff`")

    tail = goalsmod.pinned_tail(chain)
    for g in chain:
        if g.md.is_file() and g.pinned_last and g not in tail and g.num > live:
            problems.append(f"goal `{g.slug}` ({g.num}): its front matter says `position: last` and "
                            f"{len(chain) - g.num} goal(s) sit behind it -- `python tools/chain.py "
                            f"--move {g.num} --end` puts it back")

    if README.is_file():
        readme = README.read_text(encoding="utf-8")
        for g in chain:
            if not generated(g) and f"{g.stem}.md" not in readme:
                notes.append(f"{rel(README)}: no row for `{g.slug}` -- the table there is what a "
                             f"reader reads instead of the directory")

    # The rule that makes a renumber cheap, as a gate. A number is a position; naming a goal by one
    # writes a sentence that a later insert silently falsifies, and nothing else in this repository
    # would notice. AGENTS.md, *The schedule is the chain*, is the rule's home.
    stale = number_citations()
    if stale:
        problems.append(f"{len(stale)} place(s) name a goal by its NUMBER. A goal is named by its "
                        f"slug -- `goal `xml-tree``, never `goal 29` -- because a number is a "
                        f"position and moves whenever anything is inserted in front of it:")
        for path, line, text in stale[:15]:
            problems.append(f"    {path}:{line}  {text}")
        if len(stale) > 15:
            problems.append(f"    ... and {len(stale) - 15} more")

    for label, items in (("", problems), ("note: ", notes)):
        for item in items:
            print(f"  {label}{item}")
    if problems:
        print()
        print(f"chain: {len(problems)} problem(s), {len(notes)} note(s) over {len(chain)} goals")
        return 1
    print(f"chain: {len(chain)} goals numbered 1..{len(chain)}, all walkable"
          + (f" -- {len(notes)} note(s) above" if notes else ""))
    print("       `python tools/plan.py --check` is the other half: milestone tags and "
          "`Carried by` cells.")
    return 0


# ---------------------------------------------------------------------------------------------------


def main(argv=None):
    p = argparse.ArgumentParser(
        prog="chain.py", description=__doc__.split("\n\n")[0],
        formatter_class=argparse.RawDescriptionHelpFormatter, epilog=__doc__)
    p.add_argument("--show", type=int, metavar="N", help="one goal, its files and its stages")
    p.add_argument("--all", action="store_true", help="list generated goals too")
    p.add_argument("--new", metavar="SLUG", help="scaffold a goal and insert it")
    p.add_argument("--move", type=int, metavar="N", help="move goal N somewhere else")
    p.add_argument("--remove", type=int, metavar="N", help="delete goal N and close the hole")
    p.add_argument("--renumber", action="store_true", help="repair the numbering to 1..N")
    p.add_argument("--retitle", type=int, metavar="N", help="change goal N's slug, not its number")
    p.add_argument("--set", type=int, metavar="N", help="set goal N's milestone")
    p.add_argument("--retire", type=int, metavar="N", help="drop a walked goal's acceptance list")
    p.add_argument("--check", action="store_true", help="the gate")

    p.add_argument("--to", metavar="N|SLUG", help="the number to land on, or the new slug")
    p.add_argument("--after", type=int, metavar="N", help="land directly after goal N")
    p.add_argument("--before", type=int, metavar="N", help="land directly before goal N")
    p.add_argument("--next", action="store_true", help="land directly after the LIVE goal")
    p.add_argument("--end", action="store_true", help="land at the end of the chain")

    p.add_argument("--title", help="the goal's H1, for --new")
    p.add_argument("--milestone", help="the milestone tag")
    p.add_argument("--docker", action=argparse.BooleanOptionalAction, default=None,
                   help="whether the new goal declares a [docker] table")
    p.add_argument("--delete-files", action="store_true", help="--remove really deletes")
    p.add_argument("--force", action="store_true", help="touch the walked prefix anyway")
    p.add_argument("--dry-run", action="store_true", help="say what would happen")
    opts = p.parse_args(argv)

    chain = goalsmod.load()
    if opts.check:
        return cmd_check(chain)
    if opts.show is not None:
        return cmd_show(chain, opts.show)
    if opts.new:
        return cmd_new(chain, opts)
    if opts.move is not None:
        return cmd_move(chain, opts)
    if opts.remove is not None:
        return cmd_remove(chain, opts)
    if opts.renumber:
        return cmd_renumber(chain, opts)
    if opts.retitle is not None:
        if not opts.to:
            return die("--retitle takes --to <slug>")
        return cmd_retitle(chain, opts)
    if opts.set is not None:
        return cmd_set(chain, opts)
    if opts.retire is not None:
        return cmd_retire(chain, opts)
    return cmd_list(chain, opts.all)


if __name__ == "__main__":
    sys.exit(main())
