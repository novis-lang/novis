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

This module reads and never writes. `tools/chain.py` is the editor.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GOALS = ROOT / "docs" / "agent" / "goals"

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
    """One entry: its number, its files, and the two facts that are not either of those."""

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
    def name(self):
        """`29 xml-tree` -- how the chain used to spell it, and how the console still prints it."""
        return f"{self.num} {self.slug}"

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

    @property
    def milestone(self):
        """The milestone this goal builds inside, from the `.md`'s front matter."""
        m = FRONT_RE.match(self.md.read_text(encoding="utf-8"))
        if not m:
            return ""
        for line in m.group(1).split("\n"):
            key, sep, value = line.partition(":")
            if sep and key.strip() == "milestone":
                return value.strip()
        return ""

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
        return f"<Goal {self.name}>"


def load():
    """Every goal on disk, in the order the driver walks them.

    Sorted on the number and never on the filename: `10-program-id.md` sorts before `9-temp-sweep.md`
    as text, which is the bug that made a directory listing useless for reading the order off.
    """
    found = []
    for path in GOALS.rglob("*.md"):
        m = NAME_RE.match(path.name)
        if m:
            found.append(Goal(int(m.group(1)), m.group(2), path.parent))
    return sorted(found, key=lambda g: g.num)


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


def write_live(num):
    STATE.parent.mkdir(parents=True, exist_ok=True)
    STATE.write_text(json.dumps({"goal": num}, indent=2) + "\n", encoding="utf-8", newline="\n")


def rel(path):
    try:
        return Path(path).resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return Path(path).as_posix()
