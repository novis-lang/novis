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
    the goal's standing decisions                   always -- this is what keeps a run off BLOCKED
    the ground-rule bullets for the named ADRs      [context] rules
    the named ADR sections, sliced live             [context] adrs
    the map lines for the named modules             [context] modules
    the convention shapes the goal will write       [context] shapes
    the playbook sections that apply here           [context] playbook
    the milestone this goal builds inside           [context] milestones

Every one of those is sliced out of the live file at run time. **Nothing here is a copy**, so a
manifest cannot go stale in the way a frozen context pack would -- it can only go *wrong*, by
naming something that no longer exists, and that prints as a loud warning rather than as silence.

    python tools/orient.py              # the pack
    python tools/orient.py --audit      # + what each section cost, in bytes and approximate tokens
    python tools/orient.py --item N     # pin a specific checklist item instead of the first unticked
    python tools/orient.py --full       # ignore the manifest and print everything it could select

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

# A section is measured for --audit as it is emitted, so the report is of what was actually
# printed rather than of what the files hold.
out: list[str] = []
ledger: list[tuple[str, int]] = []
problems: list[str] = []


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


def run_marker() -> None:
    section("RUN", "git, and .loop/running")
    if RUNNING.exists():
        emit("A LOOP DRIVER HOLDS THIS TREE. Its sessions edit these files on nearly every")
        emit("iteration; do not start a by-hand pass over shared files while this says so.")
        for line in read(RUNNING).rstrip("\n").split("\n"):
            emit(f"  {line}")
        emit()
    branch = git("rev-parse", "--abbrev-ref", "HEAD") or "(unknown)"
    changed = [ln for ln in git("status", "--short").split("\n") if ln.strip()]
    emit(f"branch {branch}, {len(changed)} path(s) with uncommitted changes")
    emit(f"head   {git('log', '-1', '--oneline') or '(no commits)'}")


def run_numbers() -> None:
    """Reused from brief.py verbatim: the next free diagnostic code and ADR number are two
    lookups every session does, and the ADR one is a race if two agents both grep for it."""
    before = len(brief.out)
    brief.run_numbers()
    section("THE NEXT FREE NUMBER", "mwl-diagnostics (every `Code::new`) and docs/adr/ filenames")
    for line in brief.out[before:]:
        if line.startswith("== ") or line.startswith("-- source:"):
            continue
        emit(line)


def run_state(item_index: int | None) -> None:
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
    emit("\n".join(items[pick]).rstrip())

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

    hit = set()
    for blk in bullets:
        body = "\n".join(blk)
        for num in m.rules:
            if re.search(rf"\b{re.escape(num)}\b", body):
                emit(body.rstrip())
                hit.add(num)
                break
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
    """The traps, sliced by section OR by bullet -- see `slice_bullets` for why both."""
    if not wanted:
        return
    text = read(PLAYBOOK)
    if not text:
        warn(f"{rel(PLAYBOOK)} is missing")
        return
    section("THE TRAPS THAT APPLY HERE", f"{rel(PLAYBOOK)}, filtered to [context] playbook")
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
            emit()
            emit(body)


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


def run_closing() -> None:
    section("WHEN YOU ARE DONE", "AGENTS.md § Session workflow, steps 3-5")
    emit("  python tools/verify.py            build + test + clippy + fmt, once, at the end")
    emit("  python tools/session.py --check   what steps 4-5 still owe, measured off the tree")
    emit("  python tools/session.py --template a wrap skeleton, this tree's counts already in it")
    emit("  <Fill that skeleton in>           plan fields, playbook bullet, handoff, commits, status")
    emit("  python tools/session.py --wrap F  applies all of it, or refuses and changes nothing")
    emit()
    emit("`--template` IS the wrap file's format -- do not spend a call on `--help` to recover it,")
    emit("and `--wrap F --dry-run` names anything a filled-in one still gets wrong. That is the")
    emit("whole tail: three calls, none of them a `--help`, against the thirty-three it measured")
    emit("before the tool existed. Nothing about WHAT you write changes -- the handoff contract,")
    emit("one commit per slice, and the fixed plan field set all still hold, and `--wrap` refuses")
    emit("input that breaks them.")
    emit()
    emit("WHILE YOU WORK, read in one call, not fifty:")
    emit("  python tools/peek.py A.rs:120-160 B.rs:@symbol C.md:\"## 4\" \"crates/**/*.rs:re:pat\"")
    emit("  python tools/peek.py --locate <symbol> ...    file:line anchors, no bodies")
    emit("Both take as many targets as you have questions. Measured over a full run, sessions")
    emit("issued 3,647 tool calls and batched exactly none of them, including runs of 57")
    emit("consecutive greps -- so reach for these instead of a `grep`/`sed` at a time.")
    emit()
    emit("If this pack did not print something you needed, that is a gap in [context] in")
    emit("docs/agent/loop-goal.toml. Say which field was missing it, in the handoff.")


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
    opts = ap.parse_args()

    if not GOAL_TOML.exists():
        sys.stdout.write(
            f"orient.py: {rel(GOAL_TOML)} is missing, so there is no goal to narrow to.\n"
            "Run `python tools/brief.py` for the unscoped orientation.\n"
        )
        return 2
    m = Manifest(tomllib.loads(read(GOAL_TOML)))
    if opts.full:
        m.modules = ["crates/**", "editors/**"]

    emit("MWL -- oriented to the current goal. This is deliberately narrow: it prints what this")
    emit("goal's [context] manifest names and nothing else. `python tools/brief.py` is the wide one.")

    run_marker()
    run_state(opts.item)
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
