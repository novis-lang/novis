#!/usr/bin/env python3
"""The whole of a loop session's step 1, in one call, narrowed to the current goal.

`brief.py` answers "what is this repository" and prints every module, every guard test and the
whole plan status -- about 30k of context before a session has read a line of the code it came to
change. That is the right shape for a person arriving cold and the wrong shape for a session with
a two-line checklist item, because a goal is a *finite contained group of work*: it never needs
the whole repository, and every byte it does not need is charged to the 200k ceiling anyway.

So this script prints the same kinds of thing, selected by the goal's own `[context]` manifest in
`docs/agent/loop-goal.toml`:  # check-links:retired

    the run marker and the next free numbers        always
    the failing acceptance check, in full           always -- when the ledger names one
    the handoff's state and the current item        always
    the code at every `path:line` the item names    always -- see `run_anchors`
    the goal's standing decisions                   always -- this is what keeps a run off BLOCKED
    a named rule's own body; a named record's rules  [context] rules
    the named ADR sections, sliced live             [context] adrs
    the named spec sections, sliced live            [context] spec
    the map lines for the named modules             [context] modules
    the convention shapes the goal will write       [context] shapes
    the playbook traps, narrowed twice              [context] playbook, then the item's own paths
    the milestone this goal builds inside           [context] milestones

every one of those but `modules` and `plan` narrowed again by  [context.stage.N], N being the
stage handoff.md's `## Next group` names

The pack exists to buy **turns**, not bytes. A session's wall clock is very nearly its turn count
times a constant -- measured over one 33-session run, time-to-first-token was ~80% of a turn and
did not depend on what the turn fetched -- so a section here earns its place by removing a call a
session would otherwise make, not by being short. That is why the item's anchors are expanded
inline (they replace one `peek.py` call each) while the traps are narrowed to the item (a trap for
a file the item never opens removes no call at all).

And a goal is narrowed again, by the **stage** of it in flight. A goal runs five to twelve stages
and a session works in one: goal `lsp-server`'s stage 8 argues from ADR 0101's redaction sections, which say
nothing at all to the session writing its stage 4. So `[context]` may carry a `[context.stage.N]`
table per prose stage, `orient.py` applies the one `handoff.md`'s `## Next group` names, and the
base holds only what every stage needs. `Manifest` is where that merge lives and why it only ever
adds. A goal with no stage tables prints exactly the pack it printed before they existed.

Every one of those is sliced out of the live file at run time. **Nothing here is a copy**, so a
manifest cannot go stale in the way a frozen context pack would -- it can only go *wrong*, by
naming something that no longer exists, and that prints as a loud warning rather than as silence.

    python tools/orient.py              # the pack
    python tools/orient.py --audit      # + what each section cost, in bytes and approximate tokens
    python tools/orient.py --item N     # pin a specific checklist item instead of the first unticked
    python tools/orient.py --stage N    # price a stage the run has not reached, instead of the live one
    python tools/orient.py --full       # ignore the manifest and print everything it could select
    python tools/orient.py --goal docs/agent/goals/<goal>.toml --audit  # price a STAGED manifest

`--audit` reports. It never exits non-zero over a size, and nothing in this repository does:
see docs/agent/doc-style.md on why a length tripwire costs more than it saves.
"""

from __future__ import annotations

import argparse
import fnmatch
import functools
import json
import re
import subprocess
import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import brief  # noqa: E402  -- same directory, reused rather than reimplemented
import playbook  # noqa: E402  -- its `score`/`expand` decide which traps this item earns
import rules as rulebook  # noqa: E402  -- the rulebook library; `[context] rules` selects from it

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    try:
        import tomli as tomllib  # type: ignore
    except ModuleNotFoundError:
        sys.stderr.write(
            "orient.py needs TOML support: Python 3.11+, or `pip install tomli` on an older one.\n"
        )
        raise SystemExit(2)

ROOT = Path(__file__).resolve().parent.parent
AGENT = ROOT / "docs" / "agent"
#: A frozen decision record: `docs/decisions/NNNN.md`, since the docs migration's unit C1. That
#: is the only place a record lives; docs/adr/ keeps README.md's project-start decisions and
#: tooling-parity.md, neither of which a manifest names.
DECISIONS_DIR = ROOT / "docs" / "decisions"
#: Live, and staying: 01-core-library.md is read by a stdlib test and by check-migration.py, and
#: 02-php-migration.md is the one home of the table reference.py renders. A manifest's `spec`
#: entries name these files by number.
SPEC_DIR = ROOT / "docs" / "spec"

GOAL_TOML = AGENT / "loop-goal.toml"
GOAL_MD = AGENT / "loop-goal.md"
HANDOFF = AGENT / "handoff.md"
import goals as goalsmod  # noqa: E402  -- which goal this pack is for has one home
#: A side run's pack is built from its own three files where they sit (`goals.SIDE_ENV`).
SIDE = goalsmod.side_goal()
if SIDE:
    GOAL_TOML, GOAL_MD, HANDOFF = SIDE.toml, SIDE.md, SIDE.handoff
PLAYBOOK = AGENT / "playbook"
CONVENTIONS = AGENT / "conventions.md"
RUNNING = ROOT / ".loop" / "running"
INTERRUPTED = ROOT / ".loop" / "interrupted.json"
LEDGER = ROOT / ".loop" / "log.md"
DOCGATE = ROOT / ".loop" / "doc-gate.json"
OWNERGATE = ROOT / ".loop" / "owner-gate.json"

# A section is measured for --audit as it is emitted, so the report is of what was actually
# printed rather than of what the files hold.
out: list[str] = []
ledger: list[tuple[str, int]] = []
problems: list[str] = []

# The current item's own text, filled by `run_state` and read by `run_playbook`. The manifest is
# goal-scoped and an item is one file set inside it, so the goal decides which traps *could* apply
# and the item decides which of them are printed whole -- see `run_playbook`.
current_item: str = ""

#: Sub-lines `audit` prints under the traps row, filled by `run_playbook`. The traps section is
#: reliably the largest one, and its size is two costs added together that pull in different
#: directions: what the *goal's* manifest names, which is the goal author's to trim, and which of
#: those the *item's* paths promote to being printed whole, which changes item to item and is
#: nobody's to trim. A single row cannot tell a rising manifest from an item that happens to touch
#: a well-documented file, and an optimization pass reading only the row spent three passes reporting
#: the growth without being able to attribute it. Supervisor-facing only -- these lines are
#: appended after the pack, so the pack a session reads is byte-identical with and without
#: `--audit`.
traps_detail: list[str] = []

#: The traps section's heading, named once because `audit` hangs `traps_detail` off the ledger row
#: that carries it.
TRAPS_TITLE = "THE TRAPS THAT APPLY HERE"

#: How many of the item's path-promoted playbook bullets are printed WHOLE. The rest keep their
#: one-line lead-in and their `--show` selector, so nothing goes out of reach -- see
#: `run_playbook`, which owns why the bound exists and why it is here rather than on the manifest.
#:
#: 20 against the 46 one measured item promoted: the bullets a session actually acts on are the
#: ones naming its own files, and ranking puts those first. Raise it if a handoff reports a trap
#: it needed and only found in the lead-in list -- that is the signal this number is too low, and
#: it is cheaper to read than any byte count.
PROMOTED_WHOLE = 20

#: The traps for reading a failing acceptance check -- a check filed under the wrong crate, a test
#: name the tree spells differently, a conjunction name. `triage_applies` decides when a session
#: needs them, from the driver's verdict rather than from the goal or the item: `run_playbook`
#: prints them whole then and as one line otherwise, and a manifest that also names them gets them
#: under this rule rather than twice. No goal can know in advance when its checks will need
#: reading, which is why the selector lives here and not in any manifest.
TRIAGE = "Tooling > a loop-goal.toml*"

#: A `path:line` anchor in a checklist item, which `run_state` expands into a window of the file.
#: `docs` is a root here for the same reason the code trees are: a group whose work is prose --
#: a reference page, an ADR section, a table that still says a feature has no spelling -- names the
#: paragraph it rewrites, and inlining that paragraph is exactly as useful as inlining a function
#: body. Without it `session.py`'s per-item anchor gate cannot be satisfied by a documentation
#: item at all, which is how it stood when stage 10's own reference half came up. `editors` is a
#: root for the plainer reason that the editor clients are source too: a goal working there has no
#: other path to anchor, so every item it wrote was refused.
ANCHOR_RE = re.compile(
    r"\b((?:crates|tools|tests|benches|examples|fuzz|docs|editors)/[\w./-]+\.\w+):(\d+)\b"
)

#: A path a checklist item names, which `run_playbook` narrows the traps to: a file, with or without
#: a `:line` anchor, or a directory. The directory counts because an item whose work is prose often
#: names only `docs/decisions/` or `docs/rules/security*`, and an item that names no path at all has
#: every selected trap printed whole.
ITEM_PATH_RE = re.compile(
    r"\b((?:crates|tools|tests|benches|examples|fuzz|docs|editors)/[\w./-]*[\w-])"
)

#: Lines of a file printed either side of an anchor. Wide enough to hold a signature and the top
#: of a body, narrow enough that six anchors cost less than the six `peek.py` calls they replace.
ANCHOR_CONTEXT = 12


def emit(line: str = "") -> None:
    out.append(line)


def warn(msg: str) -> None:
    problems.append(msg)
    emit("")
    emit(f"!! orient.py: {msg}")


def section(title: str, source: str) -> None:
    if ledger:
        ledger[-1] = (ledger[-1][0], nbytes("\n".join(out[ledger[-1][1] :])))
    emit()
    emit(f"== {title}")
    emit(f"-- source: {source}")
    emit()
    ledger.append((title, len(out)))


def close_ledger() -> None:
    if ledger:
        ledger[-1] = (ledger[-1][0], nbytes("\n".join(out[ledger[-1][1] :])))


def nbytes(text: str) -> int:
    return len(text.encode("utf-8"))


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return ""


def rel(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def git(*args: str) -> str:
    try:
        return subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, encoding="utf-8", check=True
        ).stdout.strip()
    except (subprocess.CalledProcessError, OSError):
        return ""


# ------------------------------------------------------------------ markdown slicing
#
# One function, because every file this script reads is markdown with `##`/`###` headings and the
# thing wanted is always "that heading and everything under it until the next one at or above its
# level". Doing it once means a manifest can name a section in any of these files identically.
#
# The four parsers below are cached by their arguments, the text itself included. A caller that
# resolves many selectors against one file -- `session.py`'s playbook collision check resolves
# every bullet's, twice -- would otherwise re-parse the whole file per selector, which is quadratic
# in the playbook's size. They return tuples, so no caller can change what the next one reads.
# The cache keeps a few copies of the files it was handed for the life of the process.


HEADING_RE = re.compile(r"^(#{1,6})\s+(.*?)\s*$")


@functools.lru_cache(maxsize=16)
def headings(text: str) -> tuple[tuple[int, int, str], ...]:
    """(line index, level, title) for every heading, in order."""
    found = []
    fenced = False
    for i, line in enumerate(text.split("\n")):
        if line.lstrip().startswith("```"):
            fenced = not fenced
        if fenced:
            continue
        m = HEADING_RE.match(line)
        if m:
            found.append((i, len(m.group(1)), m.group(2)))
    return tuple(found)


@functools.lru_cache(maxsize=65536)
def normalize(title: str) -> str:
    """`### 2. Both operands must ...` -> `2 both operands must ...`, so a manifest can name a
    section as `§2`, as `2`, or by the words in its title, and all three land.

    The letter is part of the number. Without it a manifest entry of `§3a` normalizes to `3 a`,
    which is a prefix of *`### 3. A mode selects ...`* -- so `rule:config/two-modes-and-the-default-is-production`'s § 3a silently printed § 3
    instead, and the session that needed the row went and sliced it by hand. Any `### 3.` heading
    whose title happens to start with the word the suffix spells does this, so it is the number's
    own regex that has to stop splitting."""
    t = re.sub(r"[`*_]", "", title).strip().lower()
    t = re.sub(r"^(\d+[a-z]?)\s*[.)]?\s*", r"\1 ", t)
    return re.sub(r"\s+", " ", t).strip()


@functools.lru_cache(maxsize=256)
def slice_section(text: str, wanted: str) -> str | None:
    """The named heading and its body, up to the next heading at the same or a higher level."""
    lines = text.split("\n")
    hs = headings(text)
    key = normalize(wanted)
    for n, (idx, level, title) in enumerate(hs):
        norm = normalize(title)
        if norm == key or norm.startswith(key + " ") or norm.startswith(key + "."):
            end = len(lines)
            for later_idx, later_level, _ in hs[n + 1 :]:
                if later_level <= level:
                    end = later_idx
                    break
            return "\n".join(lines[idx:end]).rstrip()
    return None


def mask_code(line: str) -> str:
    """The line with every `code span` blanked to same-length filler.

    A code span cannot contain emphasis, so the `**` inside one is text. Without this, a bullet
    that *quotes* a bold phrase -- "A `**Bold phrase:**` inside a field body silently becomes an
    eighth status field." -- has its lead-in cut at the quoted `**`, leaving the two characters
    "A `" as its name. That bullet then matches 35 others as a selector, which is a bullet no
    `[context] playbook` entry can name."""
    return re.sub(r"`[^`]*`", lambda m: " " * len(m.group(0)), line)


@functools.lru_cache(maxsize=256)
def bullets(text: str, within: str | None = None) -> tuple[tuple[str, str], ...]:
    """Every `- ` bullet in the file, as (its bold lead-in, its whole text).

    A playbook bullet runs from its `- ` to the next `- ` at the same indent, the next heading,
    or the end. The lead-in is the `**Bolded sentence.**` it opens with, which is what a
    manifest names it by -- and what makes bullet-level selection possible at all."""
    body = text if within is None else (slice_section(text, within) or "")
    found, cur, lead = [], [], None
    for line in body.split("\n"):
        if re.match(r"^- ", line):
            if cur:
                found.append((lead or cur[0][2:80], "\n".join(cur).rstrip()))
            cur = [line]
            m = re.match(r"^- \*\*(.+?)\*\*", mask_code(line))
            lead = line[m.start(1):m.end(1)] if m else line[2:80]
        elif re.match(r"^#{1,6}\s", line):
            if cur:
                found.append((lead or cur[0][2:80], "\n".join(cur).rstrip()))
            cur, lead = [], None
        elif cur:
            cur.append(line)
    if cur:
        found.append((lead or cur[0][2:80], "\n".join(cur).rstrip()))
    return tuple(found)


def slice_bullets(text: str, selector: str) -> tuple[list[str], str | None]:
    """One `[context] playbook` entry -> the bullets it names, and a complaint if it named none.

    Four spellings, tried in this order:

        "Tooling"                                the whole `## Tooling` section
        "Tooling > a whole decision record"      one bullet out of it, by its bold lead-in
        "A whole decision record"                that bullet wherever it lives
        "Tooling > a loop-goal.toml*"            every bullet whose lead-in opens that way

    A selector matches the OPENING words of a lead-in, never words inside one: as a substring,
    "the end" also took a bullet about `owners.py` reading a list "to the end of the module doc".
    The trailing `*` is a claim rather than syntax -- `normalize` strips it, so it resolves the
    same here -- and `manifest_findings` is what refuses a selector that opens several lead-ins
    without it. That gate is the brake on a bullet-named selector quietly becoming a family as
    traps sharing its opening words are written down, each of them charged to every session.

    Naming bullets rather than sections is what keeps the manifest's cost tracking what the goal
    needs instead of what the file has accumulated: a section grows every time a trap is written
    down, which is the point of the file and a leak in the pack.
    """
    head, _, lead = selector.partition(">")
    head, lead = head.strip(), lead.strip()

    if not lead:
        whole = slice_section(text, head)
        if whole is not None:
            return [whole], None
        lead, head = head, ""      # not a heading: read it as a bare bullet name

    key = normalize(lead)
    hits = [body for name, body in bullets(text, head or None) if normalize(name).startswith(key)]
    if hits:
        return hits, None
    where = f" under {head!r}" if head else ""
    return [], f"names {selector!r} and no bullet{where} leads with it"


def slice_head(text: str) -> str:
    """An ADR's title, metadata block and *In short* paragraph -- everything before `## Context`,
    which is where the reasoning you only need in order to overturn the decision begins."""
    lines = text.split("\n")
    for idx, level, title in headings(text):
        if level == 2:
            return "\n".join(lines[:idx]).rstrip()
    return text.rstrip()


# ----------------------------------------------------------------------- the manifest


class Manifest:
    """`[context]` out of loop-goal.toml, narrowed to one stage. Every field is optional; an absent
    one selects nothing rather than everything, because a goal that forgot to name its modules
    should print a short pack and a loud warning, not the whole repository.

    A goal is a finite contained group of work, and a *stage* of one is finite again: goal `lsp-server` runs
    twelve, and the ADR sections its stage 8 argues from say nothing to the session writing its
    stage 4. Measured on that goal, 13,120 of 17,746 B of sliced ADR text belonged to a stage that
    was either already landed or not yet open. So `[context]` may carry per-stage tables:

        [context]                       # what a session needs whatever stage it is on
        rules = ["ide/the-rendering-has-one-home"]

        [context.stage.4]               # ... and what stage 4 needs on top of that
        rules = ["ide/an-open-document-is-its-own-entry-point"]
        adrs  = ["0099 §1"]

    **An overlay only ever adds.** Its entries are appended to the base's, deduplicated, base
    first. That is the one semantic, it applies to every field, and it is chosen over "replace"
    because the failure it can produce is a pack that carries something the stage did not need --
    which costs bytes -- rather than one missing something it did -- which costs a turn, and the
    turn is the expensive half (`orient.py`'s module doc has the measurement). The saving comes
    from keeping the *base* small, not from the overlay's ability to take anything away.

    A goal that writes no `[context.stage.N]` table behaves exactly as it did before this existed,
    which is what makes the field safe to add to sixteen queued goals one at a time.

    The stage number is the one the goal's **prose** uses -- `docs/agent/goals/<goal>.md`'s
    `## Stage N` headings, which is what `handoff.md`'s `## Next group` names. It is deliberately
    *not* a `[[check]]`'s `stage = "4 the requests"` label: those are coarser on purpose, one
    acceptance label spanning several prose stages ("Goal prose stages 4 to 9" says so in goal `lsp-server`'s
    own comment), and making the two agree would mean giving up that grouping for nothing.
    """

    FIELDS = ("modules", "rules", "adrs", "spec", "shapes", "playbook", "plan", "milestones",
              "stage")
    #: The fields an overlay may narrow. `plan` is absent because its default is two status fields
    #: every session reads, and `modules` because `context-sync.py` writes to the base list and a
    #: stage-local copy would silently stop receiving what a session edited.
    STAGE_FIELDS = ("rules", "adrs", "spec", "shapes", "playbook", "milestones")

    def __init__(self, spec: dict, stage: int | None = None):
        ctx = spec.get("context") or {}
        self.present = bool(ctx)
        self.stages = {str(k): v for k, v in (ctx.get("stage") or {}).items()}
        self.stage = str(stage) if stage is not None else None
        over = self.stages.get(self.stage) or {} if self.stage else {}
        self.applied = self.stage if self.stage in self.stages else None

        def field(name: str, default=()):
            base = list(ctx.get(name, default))
            if name in self.STAGE_FIELDS:
                base += [x for x in over.get(name, []) if x not in base]
            return base

        self.modules = field("modules")
        self.rules = [str(r) for r in field("rules")]
        self.adrs = [str(a) for a in field("adrs")]
        self.spec = [str(s) for s in field("spec")]
        self.shapes = field("shapes")
        self.playbook = field("playbook")
        self.plan = list(ctx.get("plan", ["Open now", "Blocking"]))
        self.milestones = [str(x) for x in field("milestones")]
        self.unknown = [k for k in ctx if k not in self.FIELDS]
        #: `(stage, complaint)` for anything a stage table gets wrong. Reported, never raised: a
        #: malformed overlay must not be able to stop a session opening.
        self.stage_problems: list[tuple[str, str]] = []
        for key, table in self.stages.items():
            if not isinstance(table, dict):
                self.stage_problems.append((key, "is not a table of fields"))
                continue
            if not key.isdigit():
                self.stage_problems.append((key, "is not a stage number"))
            for name in table:
                if name not in self.STAGE_FIELDS:
                    self.stage_problems.append(
                        (key, f"names `{name}`, which a stage table may not narrow -- "
                              f"one of {', '.join(self.STAGE_FIELDS)}"))

    def merged_for(self, key: str) -> dict:
        """One stage table's own entries, for the auditors that check every stage rather than the
        one in flight -- `chain.py --check` must catch a bad selector in stage 9 before the run
        reaches stage 9, not when it gets there at 3am."""
        table = self.stages.get(key)
        return table if isinstance(table, dict) else {}


# --------------------------------------------------------------------------- sections


def last_acceptance() -> tuple[str, str] | None:
    """The driver's own verdict on the last session, read back out of `.loop/log.md`.

    Returns `(session, failure)` -- `failure` empty for a run where every check passed --
    or `None` when the ledger holds no acceptance result at all.

    The driver checks the whole of `loop-goal.toml` after every session and stops the run
    the moment nothing fails (`loop.py`'s `if not fail: break`), but it writes the verdict
    only here. A session therefore cannot see a red check unless it is the one its own
    handoff group happens to name -- which is how `abi-probe` stayed red across sessions
    0055, 0056 and 0057 while each of them worked on something else. Printing it is the
    whole fix; the ledger is already on disk and costs nothing to read.
    """
    if not LEDGER.is_file():
        return None
    session, cost, fail = "", False, ""
    found = None
    for line in read(LEDGER).split("\n"):
        entry = re.match(r"- (\d{4}) ", line)
        if entry:
            session, cost, fail = entry.group(1), False, ""
            continue
        body = line.strip()
        # `goal cost:` is written for every acceptance run, `goal check:` only for a failing
        # one -- so the pair is what distinguishes "passed whole" from "never ran". The lines
        # right under a `goal check:` that name more red checks -- `also red:` from a sweep that
        # ran past its first red, `(and N later fixture(s) red ...)` from a fixture tier -- are
        # part of the same verdict.
        if body.startswith("goal cost:"):
            cost = True
        elif body.startswith("goal check:"):
            fail = body[len("goal check:"):].strip()
        elif fail and (body.startswith("also red:") or body.startswith("(and ")):
            fail += "\n" + body
        if cost and session:
            found = (session, fail)
    return found


#: The stage label `loop.py` writes into a failing check's ledger line -- `<check> [4 the lowering]:
#: <detail>` -- the earliest-stage failure first, which is the one `triage_applies` reads.
FAIL_STAGE = re.compile(r"\[(\d+)([^\]]*)\]:")


def triage_applies() -> bool:
    """Whether the driver's last verdict is one the `TRIAGE` traps help read.

    A red verdict is not enough, because a goal's acceptance list is red for nearly all of its
    life: a stage the goal has not finished has checks nothing has satisfied yet, which is open
    work and not a puzzle. What the traps diagnose is a check that should already pass -- one in
    the carried floor (a stage `0`-something, or one naming `floor`, as `loop.py` classes them), or
    in a stage before the one the handoff is working in. A fixture not yet written (`is missing`)
    and a check in the current or a later stage are the ordinary state, and neither needs them.

    A `[[check]]`'s label may group several prose stages under one number, so the comparison is
    approximate; while a group is unfinished its checks are open work whichever number it carries.
    With nothing to compare -- a handoff naming no stage, a failure carrying no label -- it prints."""
    verdict = last_acceptance()
    if verdict is None or not verdict[1]:
        return False
    fail = verdict[1]
    m = FAIL_STAGE.search(fail)
    if m is None:
        return " is missing -- " not in fail
    if m.group(1).startswith("0") or "floor" in m.group(2):
        return True
    now = current_stage()
    return now is None or int(m.group(1)) < now


def doc_gate_failure() -> tuple[str, str] | None:
    """The rustdoc gate's standing verdict, out of `.loop/doc-gate.json`, or `None` when green.

    `tools/loop.py` runs `bun nv verify --doc` only on an acceptance sweep that would reach the
    goal, and holds the goal open while it is red. It has to be printed here because no session's
    own verification will mention it, and the comment that broke it may be as old as the goal."""
    return gate_failure(DOCGATE)


def owner_gate_failure() -> tuple[str, str] | None:
    """The owner gate's standing verdict, out of `.loop/owner-gate.json`, or `None` when green.

    `tools/loop.py`'s `owner_gate` runs `owners.py --closes <slug>` and `playbook.py --closes
    <slug>` on the same sweep as the rustdoc gate and holds the goal open while a gap still names
    it; its docstring is the argument. Printed here for the same reason the rustdoc gate is: no
    session's own verification asks it."""
    return gate_failure(OWNERGATE)


def gate_failure(path: Path) -> tuple[str, str] | None:
    """A goal-end gate's `{failed, session}` file, as (session, finding), or `None` when green."""
    try:
        state = json.loads(read(path))
    except ValueError:
        return None
    if not isinstance(state, dict):
        return None
    failed = str(state.get("failed") or "").strip()
    if not failed:
        return None
    return str(state.get("session") or "").strip() or "an earlier session", failed


CHECK_FIELD = re.compile(r'^(name|file|stage) = "(.*)"\s*$')
# One block over 60 lines out of 243 on 2026-09-05, and the median is 13 -- so this truncates almost
# nothing while capping what a pathological comment can spend.
CHECK_BLOCK_LINES = 60


def check_blocks(text: str) -> list[dict]:
    """Every `[[check]]` in an acceptance list, with the comment header written above it.

    A block runs from its `[[check]]` line to the last line before the next table that is not part
    of that table's header, and the header is the unbroken run of comment lines directly above it.
    The header is worth carrying because the stage banners live there: a check printed on its own
    loses the paragraph saying what its whole stage is for."""
    lines = text.split("\n")
    blocks: list[dict] = []
    for i, ln in enumerate(lines):
        if ln.strip() != "[[check]]":
            continue
        stop = next((j for j in range(i + 1, len(lines)) if lines[j].startswith("[")), len(lines))
        last = stop - 1
        while last > i and (not lines[last].strip() or lines[last].lstrip().startswith("#")):
            last -= 1
        top, h = i, i - 1
        while h >= 0 and not lines[h].strip():
            h -= 1
        while h >= 0 and lines[h].lstrip().startswith("#"):
            top, h = h, h - 1
        fields: dict[str, str] = {}
        for body in lines[i : last + 1]:
            f = CHECK_FIELD.match(body)
            if f and f.group(1) not in fields:
                fields[f.group(1)] = f.group(2)
        blocks.append({"fields": fields, "top": top, "last": last, "lines": lines[top : last + 1]})
    return blocks


def locate_check(fail: str) -> list[dict]:
    """The block(s) the driver wrote a `goal check:` line from, found by exact string match.

    The ledger line is `f"{label}: {why}"` and `tools/loop.py` builds that label two ways only --
    `f"{c['name']} [{stage}]"` for a cargo check and `f"{leg.name} {c['file']} [{stage}]"` for a
    program one. So the match here is the reverse: reconstruct each block's label from its own
    `name`/`file`/`stage` and ask whether the ledger line starts with it. Nothing guesses at where a
    stage ends or matches a name loosely, which matters because this list is the run's stop path and
    printing a *neighbouring* check as the failing one is worse than printing none.

    Ambiguity is real -- three labels in the list are carried by two checks each -- so this returns
    every hit and the caller reports rather than resolves it. The live goal is the right file even
    under `--goal`: the verdict came out of the ledger, which is the live run's."""
    if not fail or not GOAL_TOML.is_file():
        return []
    hits = []
    for b in check_blocks(read(GOAL_TOML)):
        f = b["fields"]
        stage = f.get("stage", "?")
        if "name" in f:
            found = fail.startswith(f"{f['name']} [{stage}]: ")
        elif "file" in f:
            head = rf"^\S+ {re.escape(f['file'])} \[{re.escape(stage)}\]: "
            found = re.match(head, fail) is not None
        else:
            found = False
        if found:
            hits.append(b)
    return hits


def emit_check_block(fail: str) -> None:
    """Print the failing check itself, under the verdict, so no session goes and finds it.

    Measured at roughly 24 head calls across 15 sessions before this existed, all one shape: a
    session greps its own failing test name under `docs/agent/`, then walks 40-line windows of a
    3,955-line TOML, two to four times, to reach the block the verdict above already named."""
    hits = locate_check(fail)
    if not hits:
        return
    if len(hits) > 1:
        shown = hits[:4]
        where = " ".join(
            f"'{rel(GOAL_TOML)}:{b['top'] + 1}-{b['last'] + 1}'" for b in shown
        )
        more = "" if len(hits) == len(shown) else f" (of {len(hits)}; the rest carry it too)"
        emit()
        emit(f"{len(hits)} checks carry that exact label, so which of them failed does not follow")
        emit(f"from the ledger line. `python tools/peek.py {where}`{more}")
        emit("prints them in one call -- the one naming what failed above is yours.")
        return
    b = hits[0]
    anchor = f"{rel(GOAL_TOML)}:{b['top'] + 1}-{b['last'] + 1}"
    emit()
    emit(f"That check is {anchor}, and it is printed here in full -- it is what this")
    emit("session exists to turn green, so do not go and find it. Any comment above the")
    emit("`[[check]]` line is the stage's own header, and says what the whole stage is for:")
    emit()
    for line in b["lines"][:CHECK_BLOCK_LINES]:
        emit(f"  {line}" if line.strip() else "")
    if len(b["lines"]) > CHECK_BLOCK_LINES:
        rest = len(b["lines"]) - CHECK_BLOCK_LINES
        emit(f"  ... {rest} more line(s) -- `python tools/peek.py '{anchor}'` for the whole block.")
    emit_stage_siblings(b)


#: How many of a stage's other checks are named under the failing one. A stage is a handful of
#: checks by construction; the floor is not a stage in that sense and is excluded outright below.
STAGE_SIBLINGS = 12

#: Lines of each sibling's own `[[check]]` body. A check is a dozen lines at most; this bounds the
#: pathological one rather than trimming the ordinary one.
SIBLING_BLOCK_LINES = 14


def emit_stage_siblings(hit: dict) -> None:
    """Name the rest of the failing check's STAGE, so the session sees its whole red edge.

    Measured over one 39-session run: 34 of the 44 head calls that re-read an orientation source
    went to `loop-goal.toml`, and every one of them had the same shape -- a blind `sed -n
    '4590,4700p'` around the check printed above, or a `re:stage = ` sweep. The pack was telling
    sessions "do not go and find it" while printing one `[[check]]` of a stage that has three, so
    what they were going to find was the SIBLINGS: what else this stage asserts, and therefore
    what closing it actually means.

    One line each, not the block: the failing one is printed whole above because it is what the
    session must satisfy exactly, and the others are context for reading it. The floor stage is
    skipped -- it is the previous goal's entire acceptance list carried verbatim, hundreds of
    checks, and it is not a stage a session works inside."""
    stage = str(hit["fields"].get("stage", ""))
    if not stage or "floor" in stage.lower() or stage.startswith("0"):
        return
    siblings = [
        b for b in check_blocks(read(GOAL_TOML))
        if str(b["fields"].get("stage", "")) == stage and b["top"] != hit["top"]
    ]
    if not siblings:
        return
    emit()
    emit(f"The rest of stage {stage!r} -- {len(siblings)} more check(s). The stage goes green")
    emit("only when these do too, so they are what closing the one above actually means:")
    for b in siblings[:STAGE_SIBLINGS]:
        emit()
        emit(f"  -- {rel(GOAL_TOML)}:{b['top'] + 1}")
        # The `[[check]]` onwards, not the comment header: the header is the stage banner, and
        # it was already printed above with the failing check. `fields` cannot serve here --
        # `CHECK_FIELD` captures the text after `=`, which for a multi-line `want = [` is `[`.
        body = [ln for ln in b["lines"] if not ln.lstrip().startswith("#")]
        for line in body[:SIBLING_BLOCK_LINES]:
            emit(f"  {line}" if line.strip() else "")
        if len(body) > SIBLING_BLOCK_LINES:
            emit(f"  ... {len(body) - SIBLING_BLOCK_LINES} more line(s)")
    if len(siblings) > STAGE_SIBLINGS:
        emit(f"  ... and {len(siblings) - STAGE_SIBLINGS} more check(s) in this stage.")


def run_marker() -> None:
    section(
        "RUN",
        "git, .loop/running, .loop/interrupted.json, .loop/log.md and the failing check itself",
    )
    if RUNNING.exists():
        emit("A LOOP DRIVER HOLDS THIS TREE. Its sessions edit these files on nearly every")
        emit("iteration; do not start a by-hand pass over shared files while this says so.")
        for line in read(RUNNING).rstrip("\n").split("\n"):
            emit(f"  {line}")
        emit()
    if INTERRUPTED.exists():
        # Written by loop.py when a session was cut off with work still in the tree -- a usage
        # window closing mid-slice, or a CLI that died. The uncommitted paths below are that
        # session's unfinished slice, and without this line they look like the starting state.
        try:
            cut = json.loads(read(INTERRUPTED))
        except ValueError:
            cut = {}
        files = [str(f) for f in cut.get("files", []) if str(f).strip()]
        emit(f"THE PREVIOUS SESSION WAS CUT OFF at {cut.get('when', 'an unrecorded time')} --")
        emit(f"{cut.get('why', 'reason unrecorded')}.")
        emit("Everything it committed stands; one commit per slice is what buys that. The paths")
        emit("below are the slice it was in the MIDDLE of.")
        if cut.get("swept"):
            # The driver commits an unfinished slice rather than leaving it in the tree, so this
            # is a commit to read and continue -- not a diff to reconstruct. `mark_interrupted`
            # in loop.py owns why.
            emit(f"THE DRIVER COMMITTED THEM as {str(cut.get('head', ''))[:9]}, so the tree you")
            emit("open is clean. That commit has NOT been verified. Read it first, then continue")
            emit("it, amend it or revert it -- do not start new work on top of it, and do not")
            emit("assume the handoff describes it, because it was never written.")
        else:
            emit("They are UNCOMMITTED. Read them first and either finish that slice or revert")
            emit("it -- do not start new work on top of it, and do not assume the handoff")
            emit("describes them, because it was never written.")
        for f in files[:20]:
            emit(f"  {f}")
        if len(files) > 20:
            emit(f"  ... and {len(files) - 20} more")
        emit()
    branch = git("rev-parse", "--abbrev-ref", "HEAD") or "(unknown)"
    changed = [ln for ln in git("status", "--short").split("\n") if ln.strip()]
    emit(f"branch {branch}, {len(changed)} path(s) with uncommitted changes")
    emit(f"head   {git('log', '-1', '--oneline') or '(no commits)'}")
    verdict = last_acceptance()
    if verdict is not None:
        session, fail = verdict
        emit()
        if not fail:
            emit(f"The driver's last acceptance check, after session {session}, passed whole.")
        else:
            emit(f"THE DRIVER'S LAST ACCEPTANCE CHECK FAILED, after session {session}:")
            for ln in fail.split("\n"):
                emit(f"  {ln}")
            emit("The run ends only when every check in loop-goal.toml passes, and nothing else")
            emit("shows a session this one -- the driver writes it to the ledger and moves on.")
            emit("The first line is the EARLIEST-STAGE failing check, so it is the one that can be")
            emit("closed without three other stages landing first. On the sweep a goal is reached")
            emit("on, every other red check follows on an `also red:` line of its own: close all")
            emit("of them this session, because the next sweep is the same length whatever is left.")
            emit()
            emit("Two failures read differently, and getting them the wrong way round is the")
            emit("expensive mistake here:")
            emit("  * A check whose artefact IS NOT WRITTEN YET -- a test that 'did not run', a")
            emit("    fixture whose member does not exist, an `unrecognized subcommand` -- is an")
            emit("    item still open. It is the ordinary state of a goal in progress. It does")
            emit("    NOT outrank the handoff's next group; if the group below is the work that")
            emit("    leads to it, do the group.")
            emit("  * A check that USED TO PASS is a regression and outranks new work outright.")
            emit("The ledger in .loop/log.md says which: the same line repeating session after")
            emit("session is the first kind, and it is not an alarm.")
            emit_check_block(fail)
    # Below the acceptance verdict on purpose: a red check is a regression and outranks these.
    gate = doc_gate_failure()
    if gate is not None:
        since, why = gate
        emit()
        emit(f"THE RUSTDOC GATE IS RED, as of session {since}:")
        emit(f"  {why}")
        emit("The driver runs it only on a sweep where every acceptance check passed, and the goal is")
        emit("not reached while it is red. `bun nv verify --doc` is the whole check, and")
        emit("rustdoc names the file and the line. Fix every finding, run `--doc` until it is green,")
        emit("and say so in the handoff.")
    gate = owner_gate_failure()
    if gate is not None:
        since, why = gate
        emit()
        emit(f"THE OWNER GATE IS RED, as of session {since}:")
        emit(f"  {why}")
        emit("The driver runs it only on a sweep where every acceptance check passed, and the goal is")
        emit("not reached while a gap still names it. `python tools/owners.py --closes <slug>` and")
        emit("`python tools/playbook.py --closes <slug>` list each one. Build it and delete its item,")
        emit("strike it as a stated bound in the module's own prose, or re-tag it to a milestone whose")
        emit("plan states the scope -- a tag is not a build -- then say so in the handoff.")


def run_numbers() -> None:
    """Reused from brief.py verbatim: the next free diagnostic code and ADR number are two
    lookups every session does, and the ADR one is a race if two agents both grep for it."""
    before = len(brief.out)
    brief.run_numbers()
    section("THE NEXT FREE NUMBER", "nvs-diagnostics (every `Code::new`) and docs/decisions/ filenames")
    for line in brief.out[before:]:
        if line.startswith("== ") or line.startswith("-- source:"):
            continue
        emit(line)


def run_anchors(item: str) -> None:
    """The code at every `path:line` the current item names, inline.

    An item is written with anchors precisely so that a session does not have to re-derive them,
    and then every session spends one `peek.py` call per anchor arriving at what the anchor
    already identified. Those are the same bytes either way -- but a call is a *turn*, and a
    turn's time-to-first-token is ~80% of its clock and independent of what it fetches. Measured
    over one 33-session run, `head` (the calls before the first edit) was 12 a session against an
    orientation that had already been piped in.

    So the pack pays the bytes here and the session keeps the turns. Anchors that resolve to the
    same window are printed once; one that no longer resolves is a loud warning, because a stale
    anchor is a handoff bug and the next session is the cheapest place to catch it."""
    seen: dict[tuple[str, int], None] = {}
    for path, line in ANCHOR_RE.findall(item):
        seen.setdefault((path.replace("\\", "/"), int(line)), None)
    if not seen:
        return

    windows, missing = [], []
    for path, line in seen:
        body = read(ROOT / path)
        if not body:
            missing.append(f"{path}:{line}")
            continue
        lines = body.split("\n")
        if line > len(lines):
            missing.append(f"{path}:{line} (the file has {len(lines)} lines)")
            continue
        lo = max(1, line - ANCHOR_CONTEXT)
        hi = min(len(lines), line + ANCHOR_CONTEXT)
        # Overlapping anchors in one file collapse, so two anchors twenty lines apart cost one
        # window rather than two nearly identical ones.
        if windows and windows[-1][0] == path and lo <= windows[-1][2] + 1:
            windows[-1][2] = max(windows[-1][2], hi)
            continue
        windows.append([path, lo, hi])

    if not windows and not missing:
        return
    section(
        "THE CODE YOUR ITEM ANCHORS",
        f"{len(windows)} window(s) at the `path:line` the item names -- do not peek these again",
    )
    for path, lo, hi in sorted(windows):
        lines = (read(ROOT / path) or "").split("\n")
        emit(f"----- {path}:{lo}-{hi}")
        for n in range(lo, hi + 1):
            emit(f"{n:>5}  {lines[n - 1]}")
        emit()
    for gone in missing:
        warn(f"{rel(HANDOFF)}'s item anchors {gone}, which does not resolve -- the anchor is stale")


#: `Stage 4`, `stage 4:`, `stages 4-5` -- the number the goal's prose uses, wherever the handoff's
#: `## Next group` line names it. Written this loosely on purpose: the handoff is prose a session
#: writes, and every one of the last 25 revisions named its stage in a slightly different shape.
STAGE_IN_GROUP = re.compile(r"\bstages?\s+(\d+)", re.I)


def current_stage() -> int | None:
    """The prose stage the handoff's `## Next group` is working in, or `None`.

    Read here rather than out of `run_state` so the manifest can be built before anything is
    emitted -- the pack's very first section already depends on it. `None` means the handoff named
    no stage, and that is not an error: the manifest falls back to its base `[context]`, which is
    exactly the pack a goal with no stage tables gets. Failing *open* is deliberate, because the
    cost of guessing the stage wrong is a session missing the rule it came to work against.
    """
    text = read(HANDOFF)
    if not text:
        return None
    group = next(
        (slice_section(text, t) for _, lvl, t in headings(text)
         if lvl == 2 and normalize(t).startswith("next group")),
        None,
    )
    if not group:
        return None
    m = STAGE_IN_GROUP.search(group)
    return int(m.group(1)) if m else None


def run_state(item_index: int | None) -> None:
    global current_item
    text = read(HANDOFF)
    if not text:
        warn(f"{rel(HANDOFF)} is missing or empty -- there is no state to hand over")
        return
    section("WHERE THE WORK STANDS", f"{rel(HANDOFF)} (## State, and the current item in full)")

    state = slice_section(text, "State")
    if state:
        emit(state)
    else:
        warn(f"{rel(HANDOFF)} has no `## State` heading -- see docs/agent/session-prompt.md")

    group = next(
        (slice_section(text, t) for _, lvl, t in headings(text)
         if lvl == 2 and normalize(t).startswith("next group")),
        None,
    )
    if not group:
        warn(f"{rel(HANDOFF)} has no `## Next group` heading -- nothing names what to do next")
        return

    lines = group.split("\n")
    # Everything before the first checklist entry is the group's own framing -- which file set
    # the slices share, and in what order. That is the sentence that decides whether a second
    # slice is affordable, so it is never trimmed.
    first_item = next((i for i, ln in enumerate(lines) if re.match(r"^\s*- \[", ln)), len(lines))
    emit()
    emit("\n".join(lines[:first_item]).rstrip())

    items, current = [], []
    for ln in lines[first_item:]:
        if re.match(r"^\s*- \[", ln):
            if current:
                items.append(current)
            current = [ln]
        elif current:
            current.append(ln)
    if current:
        items.append(current)

    unticked = [i for i, blk in enumerate(items) if not re.match(r"^\s*- \[[xX]\]", blk[0])]
    if not items:
        warn(f"{rel(HANDOFF)}'s `## Next group` has no `- [ ]` checklist items")
        return
    if not unticked:
        emit()
        emit("Every item in the group is ticked. The handoff's next-group line is the goal now;")
        emit("if it names nothing further, pick from ## Backlog and say so in the handoff.")
        return

    pick = unticked[0] if item_index is None else item_index - 1
    if pick not in range(len(items)):
        warn(f"--item {item_index} is out of range: the group has {len(items)} item(s)")
        pick = unticked[0]

    emit()
    emit(f"-- YOUR ITEM ({pick + 1} of {len(items)}), in full:")
    emit()
    current_item = "\n".join(items[pick]).rstrip()
    emit(current_item)

    rest = [i for i in unticked if i != pick]
    if rest:
        emit()
        emit("-- the rest of the group, one line each. Take a second only if it touches files you")
        emit("   have already loaded and the first left you well short of the ceiling:")
        for i in rest:
            head = re.sub(r"^\s*- \[.\]\s*", "", items[i][0]).strip()
            emit(f"   [{i + 1}] {brief.strip_links(head)[:150]}")


def run_standing_decisions() -> None:
    """Always printed in full, and deliberately not filtered by the manifest. A standing decision
    exists to stop a session halting the run on BLOCKED, and the one it needs is exactly the one
    nobody predicted it would need."""
    text = read(GOAL_MD)
    if not text:
        warn(f"{rel(GOAL_MD)} is missing -- the loop has no stated goal")
        return
    section("THE GOAL'S STANDING DECISIONS", f"{rel(GOAL_MD)} (pre-authorized, never re-opened)")
    block = next(
        (slice_section(text, t) for _, lvl, t in headings(text)
         if lvl == 2 and normalize(t).startswith("standing decisions")),
        None,
    )
    if block:
        emit(block)
    else:
        warn(f"{rel(GOAL_MD)} has no `## Standing decisions` section -- loop-authoring.md § 4")


#: The longest list of rule ids or guard paths that is printed whole; a longer one prints as a count.
#:
#: A list is either whole or it is a count, never a sample. A record that amended two rules names
#: both, and a session can follow either; a record that amended sixty names none, because six of
#: them chosen by sort order are not the six the goal touches, and a bare id with no title beside
#: it cannot be triaged without opening the chapter anyway. The count still answers the one
#: question a session asks of a long list -- "is there a lot of this" -- and `python tools/rules.py
#: --show <id>` answers the rest in one call. A whole rule's own guards, in `run_one_rule`, are the
#: one list a session acts on directly, so that one is sampled rather than dropped past the cap.
NAMED_BEFORE_COUNT = 6


def rule_lines(book, ids: list[str]) -> None:
    """`- rule:<id> -- <title>` for each, marking a rule that is designed rather than shipped."""
    for rid in ids:
        r = book.by_id[rid]
        mark = "" if r.status == "shipped" else "  (designed)"
        emit(f"- `rule:{r.id}` -- {r.title}{mark}")


def capped(label: str, items: list[str], tail: str, indent: str = "  ") -> None:
    """One wrapped line naming at most `NAMED_BEFORE_COUNT` of `items`, then how many are left."""
    if not items:
        return
    shown = items[:NAMED_BEFORE_COUNT]
    rest = f", and {len(items) - NAMED_BEFORE_COUNT} more ({tail})" \
        if len(items) > NAMED_BEFORE_COUNT else ""
    emit(textwrap.fill(f"{label}: " + ", ".join(shown) + rest, width=100, initial_indent=indent,
                       subsequent_indent=indent, break_on_hyphens=False, break_long_words=False))


def run_rules(m: Manifest) -> None:
    """The rules this goal works inside, selected by `[context] rules`.

    An entry is one of two things, told apart by the `/` a rule id always has:

    * **a decision record number** -- `"0067"` -- which expands to the rules whose `because` names
      it: the ones it created as `rule:` token plus title, the ones it modified as ids. Until
      migration unit C2 those bullets came from the authored `docs/ground-rules.md`, selected by
      the numbers a bullet cited; that file is retired and the relation it encoded by hand is now
      the rulebook's `because`, which is also what a frozen record's `changes:` block is derived
      from.
    * **a rule id** -- `"core-classes/schema-plan"` -- which prints that one rule's **body**.

    The second form exists because the first prints only titles, and a title is a pointer rather
    than a rule. `AGENTS.md` is explicit that the fragment is the rule and the record is frozen
    rationale, so a goal that named its records and nothing else was shipping every session 9 KB of
    history and 0 bytes of the text it is held to -- and paying a call to fetch the rule anyway.
    Name the two or three rules the item is actually written against; the record numbers stay for
    the surrounding map.
    """
    section(
        "THE RULES THIS GOAL LIVES INSIDE",
        f"docs/rules/, selected by [context] rules = {m.rules or '[]'}",
    )
    emit("AGENTS.md's priority ordering and its four rules are already in your context. These are")
    emit("the rules this goal's own work sits inside. A rule named by id is printed whole, because")
    emit("the fragment IS the rule; a record number expands to the rules it created or changed.")
    emit("A short changed list is named whole; a long one is only its count, and `brief.py --where`")
    emit("routes a topic to the chapter that holds the rest.")
    emit()
    if not m.rules:
        warn("[context] rules is empty, so no rule and no decision is named as binding this goal")
        return
    try:
        book = rulebook.Rulebook()
    except Exception as exc:  # noqa: BLE001 -- a broken rulebook is a loud warning, not a crash
        warn(f"the rulebook did not load ({exc}); no rule can be selected")
        return

    for entry in m.rules:
        if "/" in entry:
            run_one_rule(book, entry)
            continue
        created = sorted([r for r in book.by_id.values() if r.because and r.because[0] == entry],
                         key=lambda r: (r.topic, r.order))
        changed = sorted([r for r in book.by_id.values() if entry in r.because[1:]],
                         key=lambda r: (r.topic, r.order))
        if not created and not changed:
            warn(f"[context] rules names {entry}, but no rule's `because` names that record "
                 f"and the rulebook has no rule with that id")
            continue
        emit(f"ADR {entry} -- {len(created)} rule(s) created, {len(changed)} changed:")
        rule_lines(book, [r.id for r in created])
        # The guards a session acts on are the ones under the rules it is held to, and those print
        # under `run_one_rule`. Across a record's created rules the union is a where-does-it-live
        # answer, so it is a count here and never a list.
        guards = {g for r in created for g in r.guarded_by}
        if guards:
            emit(f"  guarded by {len(guards)} path(s) (`rules.py --show <id>` lists a rule's own)")
        # A record that amended two rules names both, and one of them may be the rule the goal is
        # missing; a foundational record sits in the `because` of sixty and names none of them --
        # the header line above already carries that count.
        if changed and len(changed) <= NAMED_BEFORE_COUNT:
            emit(textwrap.fill("changed: " + ", ".join(f"rule:{r.id}" for r in changed), width=100,
                               initial_indent="  ", subsequent_indent="  ", break_on_hyphens=False,
                               break_long_words=False))
        emit()


def run_one_rule(book, rid: str) -> None:
    """One rule, printed the way `rules.py --show` prints it: the id, what holds it, and the body."""
    r = book.by_id.get(rid)
    if r is None:
        warn(f"[context] rules names rule:{rid}, and the rulebook has no rule with that id -- "
             f"`python tools/rules.py --list` is every one of them")
        return
    mark = "" if r.status == "shipped" else "  (designed, not yet shipped)"
    emit(f"---- rule:{r.id}{mark}")
    emit(f"     {r.title}")
    if r.because:
        emit(f"     decided in {', '.join(r.because)}"
             f"{'' if len(r.because) == 1 else ' (first created it, the rest amended it)'}")
    capped("guarded by", r.guarded_by, "`rules.py --show` lists them all", indent="     ")
    emit()
    emit(r.body())
    emit()


def adr_path(number: str) -> Path | None:
    frozen = DECISIONS_DIR / f"{number}.md"
    return frozen if frozen.is_file() else None


def strip_frontmatter(text: str) -> str:
    """A frozen record opens with a YAML block -- `status:`, `changes:` -- that is the
    rulebook's reverse index, not the decision. The pack carries the title and *In short*."""
    if not text.startswith("---\n"):
        return text
    end = text.find("\n---\n", 4)
    return text[end + 5:].lstrip("\n") if end != -1 else text


def run_adrs(m: Manifest) -> None:
    if not m.adrs:
        return
    section("THE ADR SECTIONS IN SCOPE", "docs/decisions/*.md, sliced live -- never a copy")
    emit("A section, not the file. If you need one this does not print, open the record at that")
    emit("heading and add the section to [context] adrs so the next session does not pay twice.")
    for entry in m.adrs:
        parts = entry.replace("§", " ").split()
        if not parts:
            continue
        number, wanted = parts[0], " ".join(parts[1:])
        path = adr_path(number)
        if path is None:
            warn(f"[context] adrs names ADR {number}, and docs/decisions/ has no {number}.md")
            continue
        text = strip_frontmatter(read(path))
        body = slice_head(text) if not wanted else slice_section(text, wanted)
        if body is None:
            warn(f"ADR {number} has no section matching {wanted!r} -- it was renamed or renumbered")
            continue
        emit()
        emit(f"---- {rel(path)}" + (f"  §{wanted}" if wanted else "  (In short)"))
        emit()
        emit(body)


def spec_path(number: str) -> Path | None:
    matches = sorted(SPEC_DIR.glob(f"{number}-*.md"))
    return matches[0] if matches else None


def run_spec(m: Manifest) -> None:
    """The spec sections the goal names, sliced out of `docs/spec/` the way `run_adrs` slices an ADR.

    An entry is a file number and a section -- `"01 §15"`, `"02 §3"` -- because the spec is three
    numbered files and one of them is 1,200 lines. A whole file is spelled with no section, and it
    is almost always the wrong thing to ask for.

    This field exists because two consecutive sessions writing differential cases each spent three
    calls hand-slicing § 1's *Replaces* column, wrote "the pack prints no spec section" in the
    handoff, and had nowhere to put the fix: `[context]` had no field for it, so the manifest could
    not be corrected the way a missing module or ADR section is.
    """
    if not m.spec:
        return
    section("THE SPEC SECTIONS IN SCOPE", "docs/spec/*.md, sliced live -- never a copy")
    emit("The member rosters and the PHP twins they replace. Same rule as the ADRs above: if you")
    emit("need a section this did not print, add it to [context] spec rather than slicing it twice.")
    for entry in m.spec:
        parts = entry.replace("§", " ").split()
        if not parts:
            continue
        number, wanted = parts[0], " ".join(parts[1:])
        path = spec_path(number)
        if path is None:
            warn(f"[context] spec names {number}, and docs/spec/ has no {number}-*.md")
            continue
        text = read(path)
        body = slice_head(text) if not wanted else slice_section(text, wanted)
        if body is None:
            warn(f"{rel(path)} has no section matching {wanted!r} -- it was renamed or renumbered")
            continue
        emit()
        emit(f"---- {rel(path)}" + (f"  §{wanted}" if wanted else "  (In short)"))
        emit()
        emit(body)


#: How many files a manifest's leftover patterns may put in the pack before the listing is cut
#: short. A selector is normally a handful of files; one broad enough to match hundreds is a
#: manifest bug, and printing all of them would spend the pack on the mistake rather than report it.
NAMED_FILES_MAX = 40


def named_files(patterns: list[str]) -> tuple[dict[str, list[tuple[str, str]]], set[str], int]:
    """`(group -> [(filename, summary)], the patterns that matched nothing, how many were cut)`.

    The second half of the map, and the reason a goal may name any file it touches. `crates/` and
    `editors/` are enumerated ahead of this because their modules are what most sessions read; the
    rest of the tree is not enumerated at all -- it is far too big, and almost none of it is ever
    named -- so the patterns left over after that pass are resolved against `git ls-files` instead.
    A manifest naming `tools/holes.py`, `docs/agent/playbook.md` or a `.nvst` case therefore
    resolves, and a warning now means one thing only: **nothing in the repository matches**.

    It used to mean two things, and the ambiguity cost a selector. A pattern naming a real file of
    the wrong *shape* warned in the same words as a pattern naming nothing, so an optimization pass
    read the warning, believed the files were gone, and deleted six selectors whose files were all
    on disk and all being hand-fetched by the sessions the manifest had meant to hand them to.
    """
    tracked = [p for p in git("ls-files").split("\n") if p]
    groups: dict[str, list[tuple[str, str]]] = {}
    hit: set[str] = set()
    kept = 0
    for path in tracked:
        matched = [pat for pat in patterns
                   if fnmatch.fnmatch(path, pat) or fnmatch.fnmatch(path, pat.rstrip("/") + "/**")]
        if not matched:
            continue
        hit.update(matched)
        kept += 1
        if kept > NAMED_FILES_MAX:
            continue
        group, _, name = path.rpartition("/")
        groups.setdefault(group or ".", []).append((name, brief.path_summary(ROOT / path)))
    return groups, set(patterns) - hit, max(0, kept - NAMED_FILES_MAX)


def run_map(m: Manifest) -> None:
    section(
        "THE MAP, SCOPED",
        f"what each file the goal names says it is, filtered to [context] modules "
        f"({len(m.modules)} pattern(s))",
    )
    # A crate is keyed by its bare name and lives under `crates/`; an editor package is already
    # keyed by its ROOT-relative path. One dict of `(group -> prefix)` keeps the loop below from
    # caring which it is looking at, which is the whole point: from M4B a goal's file set can be
    # TypeScript, and a session working there must orient the same way.
    groups = {**brief.crate_modules(), **brief.editor_modules()}
    prefix = {g: (g if "/" in g else f"crates/{g}") for g in groups}
    if not groups:
        warn("no `crates/*/src/**/*.rs` or `editors/*/src/**/*.ts` found -- those are the source")
        return
    if not m.modules:
        warn("[context] modules is empty, so the map is not printed at all. A goal that touches "
             "code must name the files it touches; `python tools/brief.py` prints all of them.")
        return

    shown, total, unmatched = 0, 0, set(m.modules)
    for crate, entries in groups.items():
        keep = []
        for within, summary in entries:
            total += 1
            full = f"{prefix[crate]}/{within}"
            # Every pattern that matches this module is a live pattern, not just the first one to
            # fire. Stopping at the first left a specific entry -- `nvs-stdlib/src/script.rs` sitting
            # under a broad `nvs-stdlib/src/*.rs` -- looking unmatched, and the warning below then
            # sent an optimization pass to delete a selector that was doing its job.
            hit = [pat for pat in m.modules
                   if fnmatch.fnmatch(full, pat) or fnmatch.fnmatch(full, pat.rstrip("/") + "/**")]
            if hit:
                keep.append((within, summary or "(no header doc comment)"))
                unmatched.difference_update(hit)
        if not keep:
            continue
        emit()
        emit(crate)
        width = max(len(w) for w, _ in keep)
        for within, summary in keep:
            emit(f"  {within:<{width}}  {summary}")
            shown += 1

    named, unmatched, cut = named_files(sorted(unmatched))
    extra = sum(len(entries) for entries in named.values())
    for group in sorted(named):
        entries = sorted(named[group])
        emit()
        emit(group)
        width = max(len(name) for name, _ in entries)
        for name, summary in entries:
            emit(f"  {name:<{width}}  {summary}" if summary else f"  {name}")
    if cut:
        emit()
        emit(f"...and {cut} more file(s) the manifest's patterns match, not listed. A selector this "
             f"broad is naming the tree rather than the goal's file set.")

    emit()
    also = f", plus {extra} file(s) it names outside them" if extra else ""
    emit(f"{shown} of {total} crate and editor module(s) are in scope{also}. For one that is not,")
    emit("`python tools/brief.py` prints the whole map -- and if you needed it, the manifest is")
    emit("missing a pattern.")
    for pat in sorted(unmatched):
        warn(f"[context] modules pattern {pat!r} matches no file in the tree -- it moved, or the "
             f"glob is wrong. The shape does not have to be Rust: any tracked file resolves.")


#: Shapes that only make sense together, as {shape: the one it implies}. A decision is two files --
#: the frozen record and the rule's fragment -- written in one commit, and a session handed only
#: the record's shape writes half of it and is then refused by `session.py --wrap`'s rulebook gate.
#: Warning here is the cheap end of that: the manifest is one line, and the refusal is at the point
#: where context is at its peak.
SHAPE_IMPLIES = {"A decision record": "A rule fragment"}

#: A `rule:<topic>/<slug>` token as the prose writes it, for reading a goal's own citations back.
RULE_TOKEN = re.compile(r"rule:([a-z0-9\-]+/[a-z0-9\-]+)")


def manifest_findings(path: Path) -> tuple[list[str], list[str]]:
    """`(problems, notes)` for one goal file's `[context]` block, printing nothing.

    `chain.py --check` walks every entry on the chain and, until this existed, checked only that
    the driver could *run* the acceptance list. Nothing looked at the manifest, which is the other
    half of whether the goal is walkable: five queued goals were naming shapes -- `"A core class"`,
    `"A .nvst case"` -- that `conventions.md` has no heading for, and one was naming
    `"0060 §§1,4,5"`, which slices no section at all. Each of those is a section the pack silently
    does not print, discovered by the session that opens the goal at 3am and works without it.

    **What gates and what only reports is a question of whether the target can legitimately not
    exist yet.** A `shapes` entry names a heading in a file that is already written, and a
    `playbook` selector names a bullet that is already there, so neither is ever forward-looking:
    those are problems. A `rules`, `adrs`, `spec` or `milestones` entry may name something this
    goal is about to create -- that is a normal way to write a manifest -- so those are notes.
    `modules` is not audited at all here: a goal whose first slice creates the crate is the
    ordinary case, and `orient.py`'s own run-time warning is the right place for it.
    """
    problems: list[str] = []
    notes: list[str] = []
    try:
        spec = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return problems, notes  # chain.py --check reports an unreadable goal itself
    # Every stage's entries at once, not just the one in flight. A goal walks its stages over days,
    # and a selector that resolves to nothing in stage 9 is discovered by the session that opens
    # stage 9 -- which is the failure this whole function exists to move forward in time.
    m = Manifest(spec)
    for key in sorted(m.stages, key=lambda k: (not k.isdigit(), k)):
        for name, entries in m.merged_for(key).items():
            if name in Manifest.STAGE_FIELDS:
                setattr(m, name, list(getattr(m, name)) + [x for x in entries
                                                           if x not in getattr(m, name)])
    where = rel(path)
    if not m.present:
        notes.append(f"{where}: has no `[context]` block, so a session opens on the unscoped pack")
        return problems, notes
    for key in m.unknown:
        problems.append(f"{where}: `[context] {key}` is not a field orient.py reads -- "
                        f"one of {', '.join(Manifest.FIELDS)}")
    for key, complaint in m.stage_problems:
        problems.append(f"{where}: `[context.stage.{key}]` {complaint}")

    conventions = read(CONVENTIONS)
    for name in m.shapes:
        if slice_section(conventions, name) is None:
            problems.append(f"{where}: shapes names {name!r}, and conventions.md has no such "
                            f"heading -- the shape is silently not printed")
    for shape, implied in SHAPE_IMPLIES.items():
        if shape in m.shapes and implied not in m.shapes:
            problems.append(f"{where}: shapes names {shape!r} without {implied!r}")

    playbook_text = playbook.read()
    for selector in m.playbook:
        hits, complaint = slice_bullets(playbook_text, selector)
        if complaint:
            problems.append(f"{where}: playbook selector {selector!r} -- {complaint}")
        elif len(hits) > 1 and not selector.rstrip().endswith("*"):
            problems.append(f"{where}: playbook selector {selector!r} opens {len(hits)} bullets' "
                            f"lead-ins -- name one, or end it in `*` to take every one of them")

    for entry in m.adrs:
        parts = entry.replace("§", " ").split()
        if not parts:
            continue
        number, wanted = parts[0], " ".join(parts[1:])
        target = adr_path(number)
        if target is None:
            notes.append(f"{where}: adrs names {number}, and docs/decisions/ has no {number}.md")
        elif wanted and slice_section(strip_frontmatter(read(target)), wanted) is None:
            notes.append(f"{where}: adrs entry {entry!r} matches no heading in {number}.md -- "
                         f"one entry is ONE section, so `§§1,4,5` slices nothing")
    for entry in m.spec:
        parts = entry.replace("§", " ").split()
        if not parts:
            continue
        number, wanted = parts[0], " ".join(parts[1:])
        target = spec_path(number)
        if target is None:
            notes.append(f"{where}: spec names {number}, and docs/spec/ has no {number}-*.md")
        elif wanted and slice_section(read(target), wanted) is None:
            notes.append(f"{where}: spec entry {entry!r} matches no heading in {rel(target)}")

    if m.rules:
        try:
            book = rulebook.Rulebook()
        except Exception:  # noqa: BLE001 -- rules.py --check is where a broken rulebook is reported
            book = None
        if book is not None:
            for entry in m.rules:
                if "/" in entry:
                    if entry not in book.by_id:
                        notes.append(f"{where}: rules names rule:{entry}, and the rulebook has no "
                                     f"rule with that id")
                elif not any(entry in r.because for r in book.by_id.values()):
                    notes.append(f"{where}: rules names {entry}, and no rule's `because` names it")

    # The two shapes a manifest is written in when nobody has read § 2 -- reported, never gating,
    # because both are judgement the goal's author makes and neither can stop a session opening.
    #
    # They are here because the doc alone did not work: on 2026-09-07 all sixteen queued goals held
    # 4-10 `rules` entries and ZERO rule ids between them, against a table that had said "name the
    # two or three the item is written against" since the field existed. A note in `chain.py
    # --check` is read at the moment the goal is being written, which the table is not.
    #
    # The first is stated as "your prose is held to a rule your manifest cannot reach", not as "you
    # named no rule id", so it clears the moment it is acted on and never fires on a goal that has
    # no rule to name. Goal `gap-owners` is that goal: a process gate over documents whose own standing
    # decisions say it opens no ADR, and the blunt form accused it of a shortfall it cannot fix.
    prose_text = read(path.with_suffix(".md"))
    if prose_text and m.rules:
        try:
            book = rulebook.Rulebook()
        except Exception:  # noqa: BLE001 -- rules.py --check reports a broken rulebook
            book = None
        if book is not None:
            named = {e for e in m.rules if "/" in e}
            records = {e for e in m.rules if "/" not in e}
            covered = named | {r.id for r in book.by_id.values()
                               if r.because and r.because[0] in records}
            unreached = sorted({rid for rid in RULE_TOKEN.findall(prose_text)
                                if rid in book.by_id and rid not in covered})
            if unreached:
                notes.append(
                    f"{where}: the prose is held to {len(unreached)} rule(s) no `rules` entry "
                    f"reaches -- {', '.join(unreached[:3])}"
                    f"{', …' if len(unreached) > 3 else ''}. Name the two or three each stage is "
                    f"written against as ids; loop-authoring.md § 2")
    prose = path.with_suffix(".md")
    stages = len(re.findall(r"^## Stage ", read(prose), flags=re.M)) if prose.is_file() else 0
    # Only where there is something a stage could take. A process goal that opens no ADR and names
    # four records -- goal `gap-owners` is the worked example -- has a base that IS what every stage needs,
    # and "a stage with nothing of its own needs no table" is the design rather than a shortfall.
    # A signal that cannot clear schedules a pass whether or not anything drifted.
    if stages >= 3 and not m.stages and (m.adrs or len(m.rules) >= 5):
        notes.append(f"{where}: runs {stages} stages and narrows to none of them -- every session "
                     f"reads all {stages} stages' rules and record sections "
                     f"(`[context.stage.N]`, loop-authoring.md § 2)")
    return problems, notes


def run_named_sections(title: str, source: Path, wanted: list[str], field: str) -> None:
    if not wanted:
        return
    text = read(source)
    if not text:
        warn(f"{rel(source)} is missing")
        return
    if field == "shapes":
        for shape, implied in SHAPE_IMPLIES.items():
            if shape in wanted and implied not in wanted:
                warn(f"[context] shapes names {shape!r} without {implied!r}. One decision is both "
                     f"files, in one commit, and `session.py --wrap` refuses a rulebook left "
                     f"half-written -- add {implied!r} to the manifest")
    section(title, f"{rel(source)}, filtered to [context] {field}")
    for name in wanted:
        body = slice_section(text, name)
        if body is None:
            warn(f"[context] {field} names {name!r}, and {rel(source)} has no such heading")
            continue
        emit()
        emit(body)


def run_playbook(wanted: list[str]) -> None:
    """The traps, sliced by section OR by bullet -- see `slice_bullets` for why both -- and then
    narrowed a second time, to the item actually being taken.

    The manifest is *goal*-scoped: it names every trap any of the goal's items could hit, against
    an item that may touch two files, and a session reads a handful of them. So the goal decides
    which traps are in scope, and the item decides which are printed **whole**: a bullet that
    mentions a path the item names, or the crate one lives in, is printed; the rest are listed by
    their lead-in with the `--show` that fetches one. An item that names no path at all falls back
    to printing every selected bullet, because then there is nothing to narrow against.

    The `TRIAGE` traps are scoped by neither: `triage_applies` reads the driver's verdict, and they
    are whole when it names a failing check that should already pass, one line otherwise.

    Nothing becomes unreachable either way, which is the property that matters -- a trap you
    cannot see is a trap you pay for twice."""
    if not wanted:
        return
    text = playbook.read()
    if not playbook.fragments():
        warn(f"{rel(PLAYBOOK)}/ holds no bullet")
        return

    triage, _ = slice_bullets(text, TRIAGE)
    needed = triage_applies()

    picked: list[tuple[str, str]] = []          # (selector, body), deduplicated
    # Seeded with the TRIAGE bullets, so a manifest naming one gets it under that rule alone.
    seen: set[str] = {body[:120] for body in triage}
    for name in wanted:
        found, complaint = slice_bullets(text, name)
        if complaint:
            warn(f"[context] playbook {complaint}")
            continue
        for body in found:
            # A goal that names both a section and one of its bullets gets it once.
            key = body[:120]
            if key in seen:
                continue
            seen.add(key)
            picked.append((name, body))

    terms = list(dict.fromkeys(ITEM_PATH_RE.findall(current_item)))

    if terms:
        ranked = sorted(
            ((playbook.score({"body": b, "section": n}, terms)[0], i, n, b)
             for i, (n, b) in enumerate(picked)),
            key=lambda r: (-r[0], r[1]),  # most terms matched first, then manifest order
        )
        hits = [(n, b) for s, _i, n, b in ranked if s]
        # THE PROMOTION IS BOUNDED, and this is the playbook's only brake.
        #
        # `playbook.md` went 644 KB -> 860 KB in the four days to 2026-09-06, at ~54 KB a day, and
        # its two pruning signals (`--check` for stale paths, `--dupes` for restatements) both read
        # clean throughout: they catch a bullet that is WRONG or DUPLICATED, and neither can catch
        # one that is correct, unique and simply not worth 818 bytes to every future session. So
        # the file has no brake at all, and the manifest is written once per goal against a file
        # that grew a third since the last one.
        #
        # The bound goes here rather than on the manifest because this is where the cost lands:
        # 46 promoted bullets were 42,828 B of a 97,648 B pack -- 44% of everything a session
        # reads, before it reads any code. Ranking by how many of the item's own paths a bullet
        # names puts the most specific traps first, and the overflow is not dropped: it joins the
        # lead-in list below, one line each, `--show`-able. Nothing becomes unreachable, which is
        # the property the item narrowing already had and the one worth keeping.
        whole, spilled = hits[:PROMOTED_WHOLE], hits[PROMOTED_WHOLE:]
        listed = [(n, b) for n, b in picked if (n, b) not in whole]
    else:
        whole, listed, spilled = picked, [], []

    section(
        TRAPS_TITLE,
        f"{rel(PLAYBOOK)}/, filtered to [context] playbook"
        + (f", then to the {len(terms)} path(s) your item names" if terms else ""),
    )
    at_whole = len(out)
    for _, body in whole:
        emit()
        emit(body)
    at_listed = len(out)

    if listed:
        emit()
        emit(f"-- {len(listed)} more trap(s) this GOAL names that your ITEM does not touch. One line")
        emit("   each; `python tools/playbook.py --show '<selector>'` prints one in full:")
        for name, body in listed:
            head = body.split("\n")[0]
            # `mask_code` preserves length, so a match on the masked line indexes the raw one.
            lead = re.match(r"^- \*\*(.+?)\*\*", mask_code(head))
            label = head[lead.start(1):lead.end(1)] if lead else head[2:]
            emit(f"   {name}  --  {brief.strip_links(label).strip('* ')[:110]}")

    at_triage = len(out)
    if triage and needed:
        emit()
        emit("-- The driver's last acceptance check FAILED on a check that should already pass, so")
        emit(f"   here in full are the {len(triage)} trap(s) for reading one:")
        for body in triage:
            emit()
            emit(body)
    elif triage:
        emit()
        emit(f"-- {len(triage)} trap(s) for reading a failing acceptance check are held back: nothing "
             f"the driver")
        emit("   reports failing is a floor check or one in a stage already passed.")
        emit(f"   `python tools/playbook.py --show '{TRIAGE}'` prints them all.")

    # The split the row itself cannot show -- see `traps_detail`. `unnarrowed` is the
    # counterfactual the goal author controls: every bullet the manifest selects, printed whole,
    # which is what this section cost before the item narrowing existed and what it would cost
    # again for an item that names no path.
    # Measured the way the section emits -- blank line, body -- so it is the same quantity as
    # `whole_b` and can never come out under it.
    unnarrowed = nbytes("\n".join(x for _, b in picked for x in ("", b)))
    whole_b = nbytes("\n".join(out[at_whole:at_listed]))
    # By subtraction, so the parts add up to the ledger row exactly. Measuring a tail on its own
    # loses the newline that joins it to the part before, and a one-byte remainder in a report
    # whose whole job is attribution reads as a contribution nobody named.
    listed_b = nbytes("\n".join(out[at_whole:at_triage])) - whole_b
    triage_b = nbytes("\n".join(out[at_whole:])) - whole_b - listed_b
    traps_detail.extend([
        f"    manifest names {len(wanted)} selector(s) -> {len(picked)} bullet(s), "
        f"{unnarrowed:,} B whole",
        f"    your ITEM's {len(terms)} path(s) promote {len(whole) + len(spilled)}, "
        f"of which {len(whole)} print whole: {whole_b:,} B"
        + (f" ({len(spilled)} spilled past PROMOTED_WHOLE={PROMOTED_WHOLE})" if spilled else "")
        if terms
        else f"    your ITEM names no path, so all {len(whole)} are printed in full: "
        f"{whole_b:,} B",
        f"    the other {len(listed)} cost one lead-in line each: {listed_b:,} B",
        f"    {len(triage)} failing-check trap(s) "
        + ("printed whole, a check that should pass being red" if needed else "held back")
        + f": {triage_b:,} B",
        f"    so narrowing saved {max(unnarrowed - whole_b - listed_b, 0):,} B -- the item's share",
        f"    is bounded by PROMOTED_WHOLE; trimming the manifest acts on the {unnarrowed:,} B",
    ])


def run_plan(m: Manifest) -> None:
    text = read(ROOT / "docs" / "implementation-plan.md")
    if not text:
        return
    fields = brief.parse_status_fields(text)
    picked = [(n, t) for n, t, _ in fields if n in m.plan]
    if not picked:
        return
    section("THE PLAN, THE FIELDS THIS GOAL READS", f"docs/implementation-plan.md, {m.plan}")
    for name, body in picked:
        emit(brief.wrap(name, brief.strip_links(body)))
    emit()
    emit("These fields are rewritten by `session.py --wrap`, never edited by hand.")


def run_milestones(m: Manifest) -> None:
    """The milestones this goal builds inside, sliced out of their own files.

    A milestone is 2-11 KB, which is why the whole plan was carved up in the first place: name
    the one this goal is inside and it costs its own size, not the roadmap's. Naming none prints
    nothing -- the index in `brief.py` and `plan.py --show Mn` are both one call away."""
    if not m.milestones:
        return
    import plan as planmod  # noqa: PLC0415 -- same directory, the plan's one API

    index = planmod.milestones()
    section(
        "THE MILESTONES THIS GOAL IS INSIDE",
        f"docs/plan/, filtered to [context] milestones = {m.milestones}",
    )
    for wanted in m.milestones:
        spec, _, part = wanted.partition(":")
        entry = planmod.resolve(spec, index)
        if entry is None:
            warn(f"[context] milestones names {spec!r}, which the plan's table does not list")
            continue
        if not entry["path"].exists():
            warn(f"[context] milestones names {spec!r}, whose file {entry['rel']} is missing")
            continue
        emit()
        if part == "verify":
            emit(f"-- {entry['id']} acceptance ({entry['rel']})")
            emit(brief.strip_links(planmod.verify_paragraph(entry)))
        elif part == "lead":
            emit(f"-- {entry['id']} lead ({entry['rel']})")
            emit(brief.strip_links(planmod.lead_paragraph(entry)))
        else:
            emit(f"-- {entry['id']} ({entry['rel']})")
            emit(planmod.body_of(entry))
    emit()
    emit("`Mn` is the whole milestone; `Mn:lead` and `Mn:verify` are the two paragraphs that")
    emit("usually answer the question. `python tools/plan.py --amend Mn --from <file>` rewrites")
    emit("one, and `session.py --wrap` takes a `## milestone: Mn` section for the same thing.")


def wrap_template() -> str:
    """`session.py --template`, run here rather than by the session.

    It is generated off the tree -- it carries the live conformance and ADR counts and the
    playbook's current headings -- so it cannot be pasted into this file as a constant. But
    it costs 0.15s to produce and nearly every session spent a whole tool call fetching it,
    which at the 38 calls a session of the 19-session run `run_closing` cites is a fixed 3% for
    1.6 KB of text (today's mean is the `fixed cost per session` line of `loop-stats.py`). Empty if the
    call fails, and the three-call wording below then stands as it always did.
    """
    try:
        done = subprocess.run(
            [sys.executable, str(ROOT / "tools" / "session.py"), "--template"],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            cwd=ROOT, timeout=60, check=True,
        )
        return done.stdout.strip("\n")
    except (subprocess.SubprocessError, OSError):
        return ""


def run_closing() -> None:
    template = wrap_template()
    section("WHEN YOU ARE DONE", "AGENTS.md § Session workflow, steps 3-5")
    emit("  bun nv verify --start / --wait    build + test + clippy + fmt, once, at the end")
    if template:
        emit("  <Fill in the skeleton below>      plan fields, playbook bullet, handoff, commits, status")
    else:
        emit("  python tools/session.py --template a wrap skeleton, already carrying every count the")
        emit("                                    tree has moved past, the playbook's headings, and")
        emit("                                    the `## commit:` for the docs it writes")
        emit("  <Fill that skeleton in>           plan fields, playbook bullet, handoff, commits, status")
    emit("  python tools/session.py --wrap F  applies all of it, or refuses and changes nothing")
    emit()
    if template:
        emit("TWO CALLS, and neither is a `--help`, a `--dry-run`, a `grep` of the playbook or a")
        emit("`session.py --template` -- the template IS the skeleton at the end of this section,")
        emit("generated for this tree at this commit, so its counts and headings are already the")
        emit("current ones. `--wrap` is all-or-nothing: it validates every section before it writes a")
    else:
        emit("THREE CALLS, and none of them is a `--help`, a `--dry-run` or a `grep` of the playbook.")
        emit("`--template` IS the format, and it answers off the tree what the tail used to re-derive")
        emit("by hand -- so do not grep the plan for a count or the playbook for its headings, they")
        emit("are in it. `--wrap` is all-or-nothing: it validates every section before it writes a")
    emit("byte, so a dry run only buys the same refusal a call earlier. ONE WRAP WRITES THE DOCS")
    emit("AND COMMITS THEM -- the handoff, the playbook and the plan are on disk before any")
    emit("commit is staged, and anything it wrote that no `## commit:` names joins the last one.")
    emit("There is no second call for a docs commit, and no `git add` by hand.")
    emit()
    emit("Nothing about WHAT you write changes -- the handoff contract, one commit per slice and")
    emit("the fixed plan field set all still hold, and `--wrap` refuses input that breaks them.")
    emit()
    emit("WHILE YOU WORK, read in one call, not fifty:")
    emit("  python tools/peek.py A.rs:120-160 B.rs:@symbol C.md:\"## 4\" \"crates/**/*.rs:re:pat:3\"")
    emit("  a re: target prints the matching line ALONE -- `re:pat:3`, or --context 3 for the")
    emit("  whole call, is how a heading or a `//!` line comes back with the block under it")
    emit("  python tools/peek.py --locate <symbol> ...    file:line anchors, no bodies")
    emit("  python tools/gaps.py              the next group, ranked: cases per member per class,")
    emit("                                    the PHP twins with no oracle case, the unasserted")
    emit("                                    error paths -- never an `ls tests/` plus a `grep`")
    emit("The first two take as many targets as you have questions. Measured over one 19-session")
    emit("run, a session issued 38 tool calls and carried 1.97 shell commands in each, so the")
    emit("habit is holding -- keep chaining read-only probes rather than spending a call each.")
    emit()
    emit("`target/debug/nvs.exe` IS ALREADY BUILT at the commit this session starts from -- the")
    emit("driver builds it after every acceptance check. Run it. Do not `ls` it first, and")
    emit("rebuild only once you have changed Rust yourself.")
    emit()
    emit("If this pack did not print something you needed, that is a gap in [context] in")
    emit("docs/agent/loop-goal.toml. Say which field was missing it, in the handoff.")  # check-links:retired
    if template:
        emit()
        emit("THE WRAP SKELETON -- `session.py --template` for this tree, so you do not call it.")
        emit("Fill it in, write it to one file, and hand that file to `--wrap`. Drop any section")
        emit("this session does not owe; `--wrap` says so if you dropped one it needed.")
        emit()
        for line in template.split("\n"):
            emit(line)


# ------------------------------------------------------------------------------ audit


#: Bytes per token, used only to turn this pack's size into a number a goal author can weigh.
#: The fallback is an estimate and is labelled as one; the real value is *regressed* by
#: `python tools/loop-stats.py --calibrate --write`, which fits recorded pack sizes against the
#: opening context those sessions actually measured, and writes the answer here for this to
#: read. A guess divided by 1.75 once put this pack at 50k tokens when the transcripts said the
#: entire session floor -- pack, harness prompt, tool schemas, CLAUDE.md and AGENTS.md together
#: -- was 57k, which is the kind of error that gets a manifest trimmed for no reason.
BYTES_PER_TOKEN_FALLBACK = 2.5
CALIBRATION = ROOT / "tools" / "data" / "calibration.json"

#: Calls in a session, for turning pack bytes into what the pack is *billed*. Every token of the
#: pack sits in the context of every turn, so the pack is paid once per call, not once.
CALLS_PER_SESSION = 98


def calibration() -> tuple[float, int, str]:
    """(bytes per token, calls per session, where the number came from)."""
    try:
        data = json.loads(CALIBRATION.read_text(encoding="utf-8"))
        ratio = float(data["bytes_per_token"])
        n = int(data.get("sessions", 0))
        return ratio, int(data.get("calls_per_session", CALLS_PER_SESSION)), (
            f"measured: regressed over {n} session(s), "
            f"R^2 {data.get('r_squared', 0):.3f}")
    except (OSError, ValueError, KeyError, TypeError):
        return BYTES_PER_TOKEN_FALLBACK, CALLS_PER_SESSION, (
            "ESTIMATED -- run `python tools/loop-stats.py --calibrate --write` "
            "after a run to measure it")


def audit() -> list[str]:
    ratio, calls, source = calibration()
    lines = [
        "",
        "== WHAT THIS PACK COST",
        f"-- bytes / {ratio:g}  ({source})",
        "",
    ]
    total = 0
    for title, size in ledger:
        total += size
        lines.append(f"  {title:<44}{size:>8,} B{size / ratio:>10,.0f} tok")
        if title == TRAPS_TITLE and traps_detail:
            lines.extend(traps_detail)
    lines.append(f"  {'TOTAL':<44}{total:>8,} B{total / ratio:>10,.0f} tok")
    lines.append("")
    lines.append("  The driver pipes this to the session, so it enters the context once -- and is")
    lines.append("  then re-billed on every turn, because a turn re-reads its whole context. At the")
    lines.append(f"  measured {calls} calls a session that is about "
                 f"{total / ratio * calls / 1_000_000:,.1f}M billed tokens, so trimming")
    lines.append(f"  1,000 bytes here is worth about {1000 / ratio * calls:,.0f} of them.")
    lines.append("")
    lines.append("  The largest section is usually the traps. A `[context] playbook` entry may name")
    lines.append("  one BULLET rather than a whole section -- `\"Tooling > a whole decision record\"` -- which is")
    lines.append("  what keeps this from growing every time a trap is written down.")
    lines.append("")
    lines.append("  Its indented rows split that cost in two, because only one half is yours: the")
    lines.append("  manifest's bullets are what a goal author trims, while which of them this ITEM")
    lines.append("  promotes to full text moves item to item. A traps row that rose because the")
    lines.append("  item touches a well-documented file is not an argument for naming fewer traps.")
    lines.append("")
    lines.append("  This is a number to look at when you WRITE a goal. It is not a check: nothing")
    lines.append("  here exits non-zero over a size (docs/agent/doc-style.md says why).")
    return lines


# ----------------------------------------------------------------------------- driver


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--audit", action="store_true", help="report what each section cost")
    ap.add_argument("--item", type=int, help="pin a checklist item instead of the first unticked")
    ap.add_argument("--full", action="store_true", help="ignore the manifest's narrowing")
    ap.add_argument(
        "--goal", metavar="TOML",
        help="read the [context] manifest from this file instead of the live goal -- for pricing a "
             "STAGED goal's manifest with --audit before switching to it, which is when "
             "loop-authoring.md § 2 says to look at that number",
    )
    ap.add_argument(
        "--stage", type=int, metavar="N",
        help="apply this goal stage's [context.stage.N] overlay instead of the one handoff.md's "
             "`## Next group` names -- for pricing a stage a run has not reached yet",
    )
    opts = ap.parse_args()

    goal_toml = Path(opts.goal) if opts.goal else GOAL_TOML
    if not goal_toml.exists():
        sys.stdout.write(
            f"orient.py: {rel(goal_toml)} is missing, so there is no goal to narrow to.\n"
            "Run `python tools/brief.py` for the unscoped orientation.\n"
        )
        return 2
    stage = opts.stage if opts.stage is not None else current_stage()
    m = Manifest(tomllib.loads(read(goal_toml)), None if opts.full else stage)
    if opts.full:
        m.modules = ["crates/**", "editors/**"]

    emit("Novis -- oriented to the current goal. This is deliberately narrow: it prints what this")
    emit("goal's [context] manifest names and nothing else. `python tools/brief.py` is the wide one.")
    if m.stages:
        named_by = "--stage" if opts.stage is not None else "handoff.md's `## Next group`"
        if m.applied:
            emit(f"Narrowed to STAGE {m.applied}, which {named_by} names: the base [context] plus")
            emit("that stage's own entries. If you need something it did not print, say so in the")
            emit("handoff naming the field -- see the closing block.")
        elif m.stage:
            emit(f"The handoff names stage {m.stage}, and this goal has no [context.stage.{m.stage}]")
            emit("table, so only the base [context] applies.")
        else:
            emit("This goal has per-stage tables and the handoff's `## Next group` names no stage,")
            emit("so only the base [context] applies -- the widest pack this goal can print.")

    run_marker()
    run_state(opts.item)
    run_anchors(current_item)
    run_standing_decisions()
    run_rules(m)
    run_adrs(m)
    run_spec(m)
    run_map(m)
    run_named_sections(
        "THE SHAPES YOU ARE ABOUT TO WRITE", CONVENTIONS, m.shapes, "shapes"
    )
    run_playbook(m.playbook)
    run_plan(m)
    run_milestones(m)
    run_numbers()
    run_closing()
    close_ledger()

    if not m.present:
        warn(
            f"{rel(GOAL_TOML)} has no [context] block at all, so nothing could be selected. "
            "docs/agent/loop-authoring.md § 2 has the field list."
        )
    if m.unknown:
        warn(f"[context] has unknown field(s): {', '.join(sorted(m.unknown))}")
    for key, complaint in m.stage_problems:
        warn(f"[context.stage.{key}] {complaint}")

    body = "\n".join(out).lstrip("\n")
    if opts.audit:
        body += "\n" + "\n".join(audit())
    sys.stdout.write(body + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
