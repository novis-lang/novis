#!/usr/bin/env python3
"""The chain, read off the goals directory. One home, imported by everything that walks it.

`docs/agent/goals/` **is** the schedule. A goal is `N-<slug>.md` plus, until it is retired, a
sibling `N-<slug>.toml` and `N-<slug>.handoff.md`; the number is the position, the numbers run
`1..N` with no gaps, and walking the chain is sorting on that number. There is no second file
saying what the order is, which is the whole point: an order kept in two places is an order that
drifts, and the `chain.toml` this replaced had drifted far enough that its seventh entry was the
one every other document called `carried-gaps` -- by a number that was not seven.

The tree under that directory is walked, not just its top level, because `dossier.py` emits ninety
goals at a time into `goals/dossier/` and three files each at the top level would bury the chain a
person reads. Where a goal's files sit says nothing about when it runs; the number says that.

Everything a chain entry used to carry is now either the filename or derived from the goal's own
files:

* **the position** is the number, so reordering is renaming and nothing else;
* **the paths** are the stem, so an entry cannot point at a file that is not there;
* **retired** is `N-<slug>.toml` being gone, which is exactly what retiring deletes -- the flag and
  the disk can no longer disagree, and they used to;
* **preflight** is a `[docker]` table in that `.toml`, which is where the containers a goal needs
  are already declared. A retired entry is never installed, so it needs no answer at all.

The one fact with nowhere to derive itself from is the **milestone**, which a retired goal still
has to answer because `plan.py` derives every `Carried by` cell from it. It is YAML front matter at
the top of the `.md`, the shape every record under `docs/decisions/` already uses.

**A side goal is not on the chain.** It is `side/<slug>.md` plus, until it is retired, a sibling
`<slug>.toml` and `<slug>.handoff.md` -- the same three files and the same shapes, with no number,
because nothing walks to it: a run reaches one only through `loop.py --side <slug>`, in a worktree of
its own, and `tools/side.py` owns how that run lands. `load` never returns one, so a side goal cannot
move a chain number or be installed by a switch; `load_side` and `side_goal` are its two readers.
`docs/agent/goals/README.md` § *Side goals* is the contract.

This module reads and never writes. `tools/chain.py` is the editor.
"""

from __future__ import annotations

import json
import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GOALS = ROOT / "docs" / "agent" / "goals"
#: Where side goals live. Walked by `load_side` and skipped by `load`.
SIDE = GOALS / "side"
#: Set to a side goal's slug in every process of a side run -- the driver, its sessions and every
#: tool they start -- so each of them reads that goal's files where a chain run reads the installed
#: `docs/agent/loop-goal.*` and `docs/agent/handoff.md`. `side_goal` is its one reader.
SIDE_ENV = "NOVIS_SIDE_GOAL"
#: `restart-free.md` -> "restart-free". A dot is not a slug character, so `<slug>.handoff.md` never
#: matches and a side goal is discovered by exactly one file, as a chain goal is.
SIDE_RE = re.compile(r"^([a-z0-9]+(?:-[a-z0-9]+)*)\.md$")

#: Where the run stands, as the number of the goal it has installed -- the number a person says out
#: loud, not a position into a list. It was an index into `chain.toml` for as long as the order
#: lived in a file whose entries could be reordered under a running driver; now the number *is* the
#: position, so an entry inserted ahead of the live goal changes no number the driver is holding.
STATE = ROOT / ".loop" / "chain.json"

#: `29-xml-tree.md` -> (29, "xml-tree"). The `.handoff.md` and `.toml` siblings are found from the
#: stem rather than matched, so a goal is discovered by exactly one file.
NAME_RE = re.compile(r"^(\d+)-([a-z0-9]+(?:-[a-z0-9]+)*)\.md$")

#: The front matter block, and the one key it carries.
FRONT_RE = re.compile(r"\A---\n(.*?)\n---\n", re.S)


class Goal:
    """One entry: its number, its files, and the two facts that are not either of those.

    **There is no `name`.** A goal is its `slug`, and its `num` is where the chain currently runs
    it; the two used to be joined into `"29 xml-tree"` for the console, which put a number in front
    of every line the driver printed and in the switch's commit message -- the one place a stale
    number outlives everything. Anything naming a goal names `slug`; anything reporting progress
    prints `num` beside a total, where it reads as the position it is.
    """

    def __init__(self, num, slug, folder=None):
        self.num = num
        self.slug = slug
        #: The directory holding the three files. `docs/agent/goals/` for a hand-written goal and
        #: `docs/agent/goals/dossier/` for a generated one -- `dossier.py` emits ninety-odd at a
        #: time and three files each at the top level would bury the chain a person reads. The
        #: number orders the chain across both, because the number is the position and the
        #: directory is only where the file sits.
        self.folder = Path(folder) if folder else GOALS

    @property
    def stem(self):
        return f"{self.num}-{self.slug}"

    @property
    def md(self):
        return self.folder / f"{self.stem}.md"

    @property
    def toml(self):
        return self.folder / f"{self.stem}.toml"

    @property
    def handoff(self):
        return self.folder / f"{self.stem}.handoff.md"

    @property
    def retired(self):
        """A walked entry whose acceptance list has been folded forward and deleted.

        Derived from the `.toml` being gone rather than from a flag, because that file *is* the
        acceptance list: a flag beside it could say the fold had happened when it had not.
        """
        return not self.toml.is_file()

    @property
    def files(self):
        """The files that must be on disk -- two fewer once the entry is retired."""
        return (self.md,) if self.retired else (self.md, self.toml, self.handoff)

    def front(self, name):
        """One key of the `.md`'s front matter, or `""`."""
        m = FRONT_RE.match(self.md.read_text(encoding="utf-8"))
        if not m:
            return ""
        for line in m.group(1).split("\n"):
            key, sep, value = line.partition(":")
            if sep and key.strip() == name:
                return value.strip()
        return ""

    @property
    def milestone(self):
        """The milestone this goal builds inside, from the `.md`'s front matter."""
        return self.front("milestone")

    @property
    def pinned_last(self):
        """Whether this goal says `position: last`: it runs after everything else on the chain.

        The number still is the position -- the key moves nothing by itself. It is what the two
        writers read so the goal *stays* last: `dossier.py --emit-goals` appends in front of it,
        and `chain.py --new --end` lands in front of it. `chain.py --check` fails when one is not
        where it says it is.
        """
        return self.front("position") == "last"

    @property
    def preflight(self):
        """`"docker"` when this goal's acceptance list needs a daemon, else `""`.

        Read off the `[docker]` table the goal already declares its containers in, so the fact is
        stated once. A retired entry answers `""` because it is never installed again.
        """
        if self.retired:
            return ""
        return "docker" if re.search(r"^\s*\[docker\]", self.toml.read_text(encoding="utf-8"),
                                     re.M) else ""

    @property
    def title(self):
        """The `.md`'s H1, less the `# Loop goal N — ` it opens with."""
        text = self.md.read_text(encoding="utf-8")
        text = FRONT_RE.sub("", text, count=1)
        h1 = text.split("\n", 1)[0]
        return re.sub(r"^#\s*Loop goal \d+\s*[—-]\s*", "", h1).strip()

    def __repr__(self):
        return f"<Goal {self.slug} at {self.num}>"


def load():
    """Every goal on disk, in the order the driver walks them.

    Sorted on the number and never on the filename: `10-program-id.md` sorts before `9-temp-sweep.md`
    as text, which is the bug that made a directory listing useless for reading the order off.
    """
    found = []
    for path in GOALS.rglob("*.md"):
        if SIDE in path.parents:
            continue
        m = NAME_RE.match(path.name)
        if m:
            found.append(Goal(int(m.group(1)), m.group(2), path.parent))
    return sorted(found, key=lambda g: g.num)


class SideGoal(Goal):
    """A goal no chain walks: `side/<slug>.md` and its two siblings, with no number.

    `num` is 0 so every caller that prints a position can tell a side goal from a chain entry, and
    the stem is the slug alone because there is no position to put in front of it."""

    def __init__(self, slug):
        super().__init__(0, slug, SIDE)

    @property
    def stem(self):
        return self.slug

    @property
    def title(self):
        """The `.md`'s H1, less the `# Side goal — ` it opens with."""
        text = FRONT_RE.sub("", self.md.read_text(encoding="utf-8"), count=1).lstrip("\n")
        return re.sub(r"^#\s*Side goal\s*[—-]\s*", "", text.split("\n", 1)[0]).strip()

    def __repr__(self):
        return f"<SideGoal {self.slug}>"


def load_side():
    """Every side goal on disk, retired ones included, in slug order."""
    if not SIDE.is_dir():
        return []
    return sorted((SideGoal(m.group(1)) for p in SIDE.iterdir()
                   if p.is_file() and (m := SIDE_RE.match(p.name))), key=lambda g: g.slug)


def side_goal():
    """The side goal this process belongs to, or None in a chain run and outside any run.

    Read from `SIDE_ENV`, which `loop.py --side` sets once and every child inherits. A slug that
    names no file is None too, and the driver refuses that at its door with the path it looked
    for."""
    slug = os.environ.get(SIDE_ENV, "").strip()
    if not slug or not SIDE_RE.match(f"{slug}.md"):
        return None
    goal = SideGoal(slug)
    return goal if goal.md.is_file() else None


def side_errors():
    """Every reason `side/` is not a set of side goals, one line each; empty when it is.

    A numbered file there would be skipped by `load` and walked by nobody, so it is refused rather
    than ignored; a live side goal missing a sibling could not be run; anything else in the
    directory is a file no reader knows about."""
    if not SIDE.is_dir():
        return []
    known, errors = set(), []
    for goal in load_side():
        known.update(p.name for p in (goal.md, goal.toml, goal.handoff))
        if goal.toml.is_file() != goal.handoff.is_file():
            have, lack = ((goal.toml, goal.handoff) if goal.toml.is_file()
                          else (goal.handoff, goal.toml))
            errors.append(f"side goal `{goal.slug}` has {rel(have)} but not {rel(lack)} -- a live "
                          f"side goal has both and a retired one has neither")
    for path in sorted(SIDE.iterdir()):
        if path.name in known:
            continue
        why = ("a numbered goal belongs on the chain, not under side/" if NAME_RE.match(path.name)
               else "not a side goal's .md, .toml or .handoff.md")
        errors.append(f"{rel(path)}: {why}")
    return errors


def pinned_tail(chain):
    """The goals at the end of `chain` that say `position: last`, in order. Usually none or one."""
    tail = []
    for g in reversed(chain):
        if not g.pinned_last:
            break
        tail.insert(0, g)
    return tail


def numbering_error(chain):
    """`""`, or the one line saying why these numbers are not a walk order.

    The numbers are the schedule, so a duplicate or a hole is not a tidiness question: a hole means
    a goal was deleted without closing it and `.loop/chain.json` now names a goal that is not there,
    and a duplicate means two files claim one position. `chain.py` closes both by renaming.
    """
    nums = [g.num for g in chain]
    dupes = sorted({n for n in nums if nums.count(n) > 1})
    if dupes:
        return (f"goal number(s) {', '.join(str(n) for n in dupes)} are claimed by more than one "
                f"file -- a number is a position and two files cannot hold one")
    if nums and nums != list(range(1, len(nums) + 1)):
        missing = sorted(set(range(1, (max(nums) if nums else 0) + 1)) - set(nums))
        return (f"the numbers run {nums[0]}..{nums[-1]} over {len(nums)} goal(s) and are not "
                f"1..{len(nums)}" + (f" -- {', '.join(str(n) for n in missing)} missing" if missing
                                     else "") + ". `python tools/chain.py --renumber` closes it")
    return ""


def find(chain, num):
    """The goal numbered `num`, or None."""
    return next((g for g in chain if g.num == num), None)


def live():
    """The number of the goal the run has installed, or 0 for "nothing installed yet"."""
    if not STATE.is_file():
        return 0
    try:
        state = json.loads(STATE.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return 0
    num = state.get("goal", 0)
    return num if isinstance(num, int) and num > 0 else 0


def live_slug():
    """The slug of the goal the run has installed, or `""` when nothing is.

    What a measurement taken mid-run records beside itself, so a reader can tell a chain switch
    from growth inside one goal: a new goal installs its own `[context]` manifest, and a pack that
    doubles across the switch was authored, not accumulated."""
    goal = find(load(), live())
    return goal.slug if goal else ""


def write_live(num):
    STATE.parent.mkdir(parents=True, exist_ok=True)
    STATE.write_text(json.dumps({"goal": num}, indent=2) + "\n", encoding="utf-8", newline="\n")


def rel(path):
    try:
        return Path(path).resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return Path(path).as_posix()
