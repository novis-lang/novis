#!/usr/bin/env python3
"""The whole of a loop session's step 1, in one call, narrowed to the current goal.

`brief.py` answers "what is this repository" and prints every module, every guard test and the
whole plan status -- about 30k of context before a session has read a line of the code it came to
change. That is the right shape for a person arriving cold and the wrong shape for a session with
a two-line checklist item, because a goal is a *finite contained group of work*: it never needs
the whole repository, and every byte it does not need is charged to the 200k ceiling anyway.

So this script prints the same kinds of thing, selected by the goal's own `[context]` manifest in
`docs/agent/loop-goal.toml`:

    the run marker and the next free numbers        always
    the handoff's state and the current item        always
    the code at every `path:line` the item names    always -- see `run_anchors`
    the goal's standing decisions                   always -- this is what keeps a run off BLOCKED
    the ground-rule bullets for the named ADRs      [context] rules
    the named ADR sections, sliced live             [context] adrs
    the map lines for the named modules             [context] modules
    the convention shapes the goal will write       [context] shapes
    the playbook traps, narrowed twice              [context] playbook, then the item's own paths
    the milestone this goal builds inside           [context] milestones

The pack exists to buy **turns**, not bytes. A session's wall clock is very nearly its turn count
times a constant -- measured over one 33-session run, time-to-first-token was ~80% of a turn and
did not depend on what the turn fetched -- so a section here earns its place by removing a call a
session would otherwise make, not by being short. That is why the item's anchors are expanded
inline (they replace one `peek.py` call each) while the traps are narrowed to the item (a trap for
a file the item never opens removes no call at all).

Every one of those is sliced out of the live file at run time. **Nothing here is a copy**, so a
manifest cannot go stale in the way a frozen context pack would -- it can only go *wrong*, by
naming something that no longer exists, and that prints as a loud warning rather than as silence.

    python tools/orient.py              # the pack
    python tools/orient.py --audit      # + what each section cost, in bytes and approximate tokens
    python tools/orient.py --item N     # pin a specific checklist item instead of the first unticked
    python tools/orient.py --full       # ignore the manifest and print everything it could select
    python tools/orient.py --goal docs/agent/next-goal-m4b.toml --audit   # price a STAGED manifest

`--audit` reports. It never exits non-zero over a size, and nothing in this repository does:
see docs/agent/doc-style.md on why a length tripwire costs more than it saves.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import brief  # noqa: E402  -- same directory, reused rather than reimplemented
import playbook  # noqa: E402  -- its `score`/`expand` decide which traps this item earns

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
ADR_DIR = ROOT / "docs" / "adr"

GOAL_TOML = AGENT / "loop-goal.toml"
GOAL_MD = AGENT / "loop-goal.md"
HANDOFF = AGENT / "handoff.md"
PLAYBOOK = AGENT / "playbook.md"
CONVENTIONS = AGENT / "conventions.md"
GROUND_RULES = ADR_DIR / "ground-rules.md"
RUNNING = ROOT / ".loop" / "running"
INTERRUPTED = ROOT / ".loop" / "interrupted.json"
LEDGER = ROOT / ".loop" / "log.md"

# A section is measured for --audit as it is emitted, so the report is of what was actually
# printed rather than of what the files hold.
out: list[str] = []
ledger: list[tuple[str, int]] = []
problems: list[str] = []

# The current item's own text, filled by `run_state` and read by `run_playbook`. The manifest is
# goal-scoped and an item is one file set inside it, so the goal decides which traps *could* apply
# and the item decides which of them are printed whole -- see `run_playbook`.
current_item: str = ""

#: A `path:line` anchor in a checklist item, which `run_state` expands into a window of the file.
#: `docs` is a root here for the same reason the code trees are: a group whose work is prose --
#: a reference page, an ADR section, a table that still says a feature has no spelling -- names the
#: paragraph it rewrites, and inlining that paragraph is exactly as useful as inlining a function
#: body. Without it `session.py`'s per-item anchor gate cannot be satisfied by a documentation
#: item at all, which is how it stood when stage 10's own reference half came up.
ANCHOR_RE = re.compile(
    r"\b((?:crates|tools|tests|benches|examples|fuzz|docs)/[\w./-]+\.\w+):(\d+)\b"
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


HEADING_RE = re.compile(r"^(#{1,6})\s+(.*?)\s*$")


def headings(text: str) -> list[tuple[int, int, str]]:
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
    return found


def normalize(title: str) -> str:
    """`### 2. Both operands must ...` -> `2 both operands must ...`, so a manifest can name a
    section as `§2`, as `2`, or by the words in its title, and all three land."""
    t = re.sub(r"[`*_]", "", title).strip().lower()
    t = re.sub(r"^(\d+)\s*[.)]?\s*", r"\1 ", t)
    return re.sub(r"\s+", " ", t).strip()


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


def bullets(text: str, within: str | None = None) -> list[tuple[str, str]]:
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
    return found


def slice_bullets(text: str, selector: str) -> tuple[list[str], str | None]:
    """One `[context] playbook` entry -> the bullets it names, and a complaint if it named none.

    Three spellings, tried in this order:

        "Tooling"                    the whole `## Tooling` section, as before
        "Tooling > A whole ADR"      one bullet out of it, by its bold lead-in
        "A whole ADR"                that bullet wherever it lives

    The second and third are why this exists. Measured over one run, the four whole sections a
    goal named were 31 KB -- 35% of the entire pack and its single largest section -- and a
    session reads perhaps three of their bullets. A section grows every time a trap is written
    down, which is the point of the file and a leak in the pack; naming bullets makes the
    manifest's cost track what the goal actually needs instead of what the file has accumulated.
    """
    head, _, lead = selector.partition(">")
    head, lead = head.strip(), lead.strip()

    if not lead:
        whole = slice_section(text, head)
        if whole is not None:
            return [whole], None
        lead, head = head, ""      # not a heading: read it as a bare bullet name

    key = normalize(lead)
    hits = [body for name, body in bullets(text, head or None) if key in normalize(name)]
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
    """`[context]` out of loop-goal.toml. Every field is optional; an absent one selects nothing
    rather than everything, because a goal that forgot to name its modules should print a short
    pack and a loud warning, not the whole repository."""

    FIELDS = ("modules", "rules", "adrs", "shapes", "playbook", "plan", "milestones")

    def __init__(self, spec: dict):
        ctx = spec.get("context") or {}
        self.present = bool(ctx)
        self.modules = list(ctx.get("modules", []))
        self.rules = [str(r) for r in ctx.get("rules", [])]
        self.adrs = [str(a) for a in ctx.get("adrs", [])]
        self.shapes = list(ctx.get("shapes", []))
        self.playbook = list(ctx.get("playbook", []))
        self.plan = list(ctx.get("plan", ["Open now", "Blocking"]))
        self.milestones = [str(x) for x in ctx.get("milestones", [])]
        self.unknown = [k for k in ctx if k not in self.FIELDS]


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
        # one -- so the pair is what distinguishes "passed whole" from "never ran".
        if body.startswith("goal cost:"):
            cost = True
        elif body.startswith("goal check:"):
            fail = body[len("goal check:"):].strip()
        if cost and session:
            found = (session, fail)
    return found


def run_marker() -> None:
    section("RUN", "git, .loop/running, .loop/interrupted.json and .loop/log.md")
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
        emit("below are the slice it was in the MIDDLE of. Read them first and either finish that")
        emit("slice or revert it -- do not start new work on top of it, and do not assume the")
        emit("handoff describes them, because it was never written.")
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
    if verdict is None:
        return
    session, fail = verdict
    emit()
    if not fail:
        emit(f"The driver's last acceptance check, after session {session}, passed whole.")
        return
    emit(f"THE DRIVER'S LAST ACCEPTANCE CHECK FAILED, after session {session}:")
    emit(f"  {fail}")
    emit("The run ends only when every check in loop-goal.toml passes, and nothing else")
    emit("shows a session this one -- the driver writes it to the ledger and moves on. If")
    emit("the group below does not close it, CLOSE THIS FIRST: it outranks the handoff's")
    emit("next group, and the handoff you write says what you found. A check that names a")
    emit("test that 'did not run' is an item still open and is the ordinary state of this")
    emit("goal; any other failure is a regression and outranks new work outright.")


def run_numbers() -> None:
    """Reused from brief.py verbatim: the next free diagnostic code and ADR number are two
    lookups every session does, and the ADR one is a race if two agents both grep for it."""
    before = len(brief.out)
    brief.run_numbers()
    section("THE NEXT FREE NUMBER", "nvs-diagnostics (every `Code::new`) and docs/adr/ filenames")
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


def run_rules(m: Manifest) -> None:
    text = read(GROUND_RULES)
    if not text:
        warn(f"{rel(GROUND_RULES)} is missing -- no rule bullets can be selected")
        return
    section(
        "THE RULES THIS GOAL LIVES INSIDE",
        f"{rel(GROUND_RULES)}, filtered to [context] rules = {m.rules or '[]'}",
    )
    emit("AGENTS.md's priority ordering and its four rules are already in your context. These are")
    emit("the decisions this goal's own work sits inside; the linked ADR's body is the rule.")
    emit()
    if not m.rules:
        warn("[context] rules is empty, so no decision is named as binding this goal")
        return

    # A bullet is one `- ` item and its continuation lines; it is selected when it cites one of
    # the named ADRs, whether as `(0090)` or as `[0090](0090-...)`.
    bullets, current = [], []
    for ln in text.split("\n"):
        if ln.startswith("- "):
            if current:
                bullets.append(current)
            current = [ln]
        elif current and (ln.startswith("  ") or not ln.strip()):
            current.append(ln)
        elif current:
            bullets.append(current)
            current = []
    if current:
        bullets.append(current)

    # One bullet may cite several of the goal's ADRs -- ground-rules.md is one sentence per decision
    # and a decision that amends another names both. It is printed once and credits *every* number it
    # cites: crediting only the first made the rest look uncited, and the warning then said a rule was
    # missing from a file that was printing it two lines above.
    hit = set()
    for blk in bullets:
        body = "\n".join(blk)
        cited = [num for num in m.rules if re.search(rf"\b{re.escape(num)}\b", body)]
        if cited:
            emit(body.rstrip())
            hit.update(cited)
    for num in m.rules:
        if num not in hit:
            warn(f"[context] rules names {num}, but no bullet in {rel(GROUND_RULES)} cites it")


def adr_path(number: str) -> Path | None:
    matches = sorted(ADR_DIR.glob(f"{number}-*.md"))
    return matches[0] if matches else None


def run_adrs(m: Manifest) -> None:
    if not m.adrs:
        return
    section("THE ADR SECTIONS IN SCOPE", "docs/adr/*.md, sliced live -- never a copy")
    emit("A section, not the file. If you need one this does not print, open the ADR at that")
    emit("heading and add the section to [context] adrs so the next session does not pay twice.")
    for entry in m.adrs:
        parts = entry.replace("§", " ").split()
        if not parts:
            continue
        number, wanted = parts[0], " ".join(parts[1:])
        path = adr_path(number)
        if path is None:
            warn(f"[context] adrs names ADR {number}, and docs/adr/ has no {number}-*.md")
            continue
        text = read(path)
        body = slice_head(text) if not wanted else slice_section(text, wanted)
        if body is None:
            warn(f"ADR {number} has no section matching {wanted!r} -- it was renamed or renumbered")
            continue
        emit()
        emit(f"---- {rel(path)}" + (f"  §{wanted}" if wanted else "  (In short)"))
        emit()
        emit(body)


def run_map(m: Manifest) -> None:
    section(
        "THE MAP, SCOPED",
        f"each module's own `//!` first sentence, filtered to [context] modules ({len(m.modules)} pattern(s))",
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
            for pat in m.modules:
                if fnmatch.fnmatch(full, pat) or fnmatch.fnmatch(full, pat.rstrip("/") + "/**"):
                    keep.append((within, summary or "(no header doc comment)"))
                    unmatched.discard(pat)
                    break
        if not keep:
            continue
        emit()
        emit(crate)
        width = max(len(w) for w, _ in keep)
        for within, summary in keep:
            emit(f"  {within:<{width}}  {summary}")
            shown += 1

    emit()
    emit(f"{shown} of {total} module(s) are in scope. For one that is not, `python tools/brief.py`")
    emit("prints the whole map -- and if you needed it, the manifest is missing a pattern.")
    for pat in sorted(unmatched):
        warn(f"[context] modules pattern {pat!r} matched no module -- it moved, or the glob is wrong")


def run_named_sections(title: str, source: Path, wanted: list[str], field: str) -> None:
    if not wanted:
        return
    text = read(source)
    if not text:
        warn(f"{rel(source)} is missing")
        return
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

    The manifest is *goal*-scoped: it names every trap any of the goal's items could hit, which on
    the current goal is 46 selectors and 33 KB a session, against an item that touches two files.
    A session reads a handful of them. So the goal still decides which traps are in scope, and the
    item decides which are printed **whole**: a bullet that mentions a path the item names, or the
    crate one lives in, is printed; the rest are listed by their lead-in with the `--show` that
    fetches one.

    Nothing becomes unreachable this way, which is the property that matters -- a trap you cannot
    see is a trap you pay for twice. An item that names no path at all falls back to printing
    every selected bullet, because then there is nothing to narrow against."""
    if not wanted:
        return
    text = read(PLAYBOOK)
    if not text:
        warn(f"{rel(PLAYBOOK)} is missing")
        return

    picked: list[tuple[str, str]] = []          # (selector, body), deduplicated
    seen: set[str] = set()
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

    terms = [p for p, _ in ANCHOR_RE.findall(current_item)]
    terms += playbook.HANDOFF_PATH.findall(current_item)
    terms = list(dict.fromkeys(terms))

    if terms:
        whole = [(n, b) for n, b in picked if playbook.score({"body": b, "section": n}, terms)[0]]
        listed = [(n, b) for n, b in picked if (n, b) not in whole]
    else:
        whole, listed = picked, []

    section(
        "THE TRAPS THAT APPLY HERE",
        f"{rel(PLAYBOOK)}, filtered to [context] playbook"
        + (f", then to the {len(terms)} path(s) your item names" if terms else ""),
    )
    for _, body in whole:
        emit()
        emit(body)

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
    emit("  python tools/verify.py            build + test + clippy + fmt, once, at the end")
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
    emit("  python tools/peek.py A.rs:120-160 B.rs:@symbol C.md:\"## 4\" \"crates/**/*.rs:re:pat\"")
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
    emit("docs/agent/loop-goal.toml. Say which field was missing it, in the handoff.")
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
    lines.append(f"  {'TOTAL':<44}{total:>8,} B{total / ratio:>10,.0f} tok")
    lines.append("")
    lines.append("  The driver pipes this to the session, so it enters the context once -- and is")
    lines.append("  then re-billed on every turn, because a turn re-reads its whole context. At the")
    lines.append(f"  measured {calls} calls a session that is about "
                 f"{total / ratio * calls / 1_000_000:,.1f}M billed tokens, so trimming")
    lines.append(f"  1,000 bytes here is worth about {1000 / ratio * calls:,.0f} of them.")
    lines.append("")
    lines.append("  The largest section is usually the traps. A `[context] playbook` entry may name")
    lines.append("  one BULLET rather than a whole section -- `\"Tooling > A whole ADR\"` -- which is")
    lines.append("  what keeps this from growing every time a trap is written down.")
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
    opts = ap.parse_args()

    goal_toml = Path(opts.goal) if opts.goal else GOAL_TOML
    if not goal_toml.exists():
        sys.stdout.write(
            f"orient.py: {rel(goal_toml)} is missing, so there is no goal to narrow to.\n"
            "Run `python tools/brief.py` for the unscoped orientation.\n"
        )
        return 2
    m = Manifest(tomllib.loads(read(goal_toml)))
    if opts.full:
        m.modules = ["crates/**", "editors/**"]

    emit("Novis -- oriented to the current goal. This is deliberately narrow: it prints what this")
    emit("goal's [context] manifest names and nothing else. `python tools/brief.py` is the wide one.")

    run_marker()
    run_state(opts.item)
    run_anchors(current_item)
    run_standing_decisions()
    run_rules(m)
    run_adrs(m)
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

    body = "\n".join(out).lstrip("\n")
    if opts.audit:
        body += "\n" + "\n".join(audit())
    sys.stdout.write(body + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
