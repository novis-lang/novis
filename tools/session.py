#!/usr/bin/env python3
"""AGENTS.md § *Session workflow* steps 4 and 5, as one call.

Measured over a full loop run, the tail of a session -- everything from the first verification to
the last commit -- was **33 of 98 tool calls**, and because context is at its peak by then those
turns carried **42% of the whole token bill** at roughly twice the cost of a turn at the start.
Almost none of it was thinking. Per session it was 6.0 calls touching the plan, 5.2 re-deriving
`file:line` anchors already known, 1.8 writing commit messages to temp files, and the rest
handoff, playbook, `git add`, `git commit`, `status.txt`.

None of that is a decision. It is a form being filled in, and a form is a thing a tool fills.

    python tools/session.py --wrap .agent-tmp/wrap.md

One file in, the whole tail out: the plan's fields, a playbook bullet, the handoff, one commit
per slice, and the loop's status line. Two tool calls -- the Write that produces the file, and
this -- in place of about twenty.

## The wrap file

Plain markdown. Every `## ` heading is an instruction; the text under it is that instruction's
payload. Order in the file does not matter -- the order of *application* is fixed below, so the
docs are on disk before anything is staged.

    ## plan-edit: Open now
    --- old
    conformance is 506 of the 600
    --- new
    conformance is 512 of the 600

    Change one run of words inside a field and leave the rest alone. Repeat the `--- old` /
    `--- new` pair as many times as the field moved. Each `--- old` must match the field
    exactly once -- quote it as the field READS, which is one single-spaced paragraph
    (`python tools/plan.py --get "Open now"` prints it). THIS IS THE USUAL ONE: a session
    changes a sentence of a field, not a field. A field has a ceiling (the plan's header
    comment is the home of the number, as of the aim), and an edit that leaves it both
    over the ceiling AND bigger than it was is refused, naming how much to cut. Replace
    the sentence that went stale -- an empty `--- new` drops one -- rather than adding
    after it; a field that shrinks or holds its size is always taken.

    ## plan: On disk
    What that field should now say, whole -- it is overwritten, not appended to. For a field
    you are genuinely rewriting. A big replacement that is mostly already on disk word for
    word is refused as a retype, and names `## plan-edit:` instead.

    ## milestone: M4S
    The whole body of docs/plan/m4s.md below its heading, replacing what is there. For the
    session that finishes a milestone, or the ADR slice that reopens one. It will not create
    a milestone the plan's table does not already list.

    ## playbook: Tooling
    - **A new trap, as a bullet.** Appended under that heading, never rewriting what is there.

    ## handoff
    ## State
    ...the whole handoff body, verbatim, replacing the file...

    ## commit: crates/nvs-ir/src/lower/expr.rs crates/nvs-ir/src/ir.rs
    feat(ir): the subject line

    The body, if there is one.

    ## commit: docs/implementation-plan.md docs/agent/handoff.md check-links:retired
    docs(agent): the second slice's own commit

    ## status
    CONTINUE one line saying what landed

Applied in this order, and **nothing is applied until every section validates**: plan fields,
plan edits, milestones, playbook, handoff, commits in the order written, then `status.txt`. A
`## commit:`
whose paths match nothing staged is an error before the first field is touched, not a
half-finished tail.

Because the docs are written before anything is staged, **one wrap writes the handoff, the
playbook and the plan and commits them** -- there is never a second call for that. Name them in
a `## commit:` as above (`--template` pre-fills it); anything this wrap wrote that no section
names joins the last commit rather than being left dirty, and the report says which paths it
added and to which commit.

## The rest

    python tools/session.py --check          # what step 4-5 still owes, from the tree
    python tools/session.py --counts         # conformance / differential / ADR counts, ready to paste
    python tools/session.py --wrap F --dry-run    # say what it would do, touch nothing

A commit message documents the work and nothing else. Any `Co-Authored-By`, `Signed-off-by`,
`Generated-with` or similar attribution line in a `## commit:` body is **stripped** before the
commit is made, and the count is reported. conventions.md § *A commit message* is the rule;
`tools/git-hooks/commit-msg` catches the same thing arriving by any other route.

`--check` is the one to run *before* writing the wrap file: it says which plan fields have gone
stale against the tree, whether the handoff still matches its contract, what is uncommitted, and
which links this session broke.

That last one is a refusal too. A wrap will not write a **dead link** -- one in a body it is about
to write, or one anywhere in the tree that resolved at HEAD and does not resolve now. Both are
`check-links.py`, which is CI's `docs` job and which `nv verify` deliberately does not run (its own
docstring says why), so a green verification says nothing at all about links and this is the last
moment before the push. A link that was **already** dead at HEAD is printed and refuses nothing:
that one is CI's to report and a human's to schedule, and holding this session for it would leave
verified work uncommitted over a link nobody here touched.

This tool judges no content. It refuses malformed input and it refuses to invent a plan field --
everything else it writes is what you handed it.
"""

from __future__ import annotations

import argparse
import fnmatch
import importlib.util
import json
import posixpath
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover -- 3.10 and older
    import tomli as tomllib  # type: ignore

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
AGENT = DOCS / "agent"
PLAN = DOCS / "implementation-plan.md"
#: The most a new `## playbook:` bullet may weigh, trailer included. A bullet is charged to every
#: session whose item names its file, and at a median 849 bytes the file had become the sessions'
#: changelog rather than their traps; the shape (three sentences, ~400 B) is
#: docs/agent/conventions.md § *A playbook bullet*. This is a gate on a NEW bullet only -- the one
#: moment the cost of trimming it is a sentence, not a session's tail spent shaving prose.
PLAYBOOK_BULLET_MAX = 700
HANDOFF = AGENT / "handoff.md"
RUNDIR = ROOT / ".loop"
STATUS = RUNDIR / "status.txt"
TMP = ROOT / ".agent-tmp"

#: conventions.md § *A commit message* is the home of this; it is named here so the refusal can
#: say how far over a subject is rather than only how long it is.
SUBJECT_MAX = 120

HANDOFF_REQUIRED = ["## State", "## Next group", "## Backlog"]
HANDOFF_TARGET_LINES = 60
STATUS_WORDS = ("CONTINUE", "DONE", "BLOCKED")

#: Every session's pack size, one JSON object per wrap. See `record_pack`.
PACK_LOG = RUNDIR / "pack-size.jsonl"

#: Growth in one session, in bytes, past which `record_pack` says so. Deliberately NOT a check:
#: doc-style.md § *Length targets* records that a hard size gate cost five and ten iterations a
#: session shaving prose to clear it, which is far more than the bytes were ever worth. This
#: number only decides whether one line is printed.
PACK_NOTE_AT = 1_500

sys.path.insert(0, str(Path(__file__).resolve().parent))
import plan as planmod  # noqa: E402  -- the status block's one home; never reimplemented here
import orient  # noqa: E402  -- its ANCHOR_RE is what the next pack expands, so it is what we gate on
import playbook as playbookmod  # noqa: E402  -- bullet parsing has one home and it is not here
import goals as goalsmod  # noqa: E402  -- which goal is live has one home and it is not here

#: The side goal this wrap belongs to, or None in a chain run. Its handoff is the goal's own
#: `.handoff.md` (`goals.SIDE_ENV`), and it writes no plan section (`validate`).
SIDE = goalsmod.side_goal()
if SIDE:
    HANDOFF = SIDE.handoff

#: `check-links.py` cannot be imported by name -- a hyphen is not an identifier -- and renaming it
#: would change a command that CI, `nv verify`'s module doc and the playbook all already spell. So
#: it is loaded by path. Every rule about what a link is and where it resolves lives there.
_LINKS = importlib.util.spec_from_file_location(
    "check_links", Path(__file__).resolve().parent / "check-links.py")
checklinks = importlib.util.module_from_spec(_LINKS)
_LINKS.loader.exec_module(checklinks)

#: `rules.py`, loaded the same way and for the same reason: `body_citations` resolves a `rule:`
#: token in a wrap body against the rulebook, and that file's `CITATION` is the one spelling of a
#: citation the whole tree uses.
_RULES = importlib.util.spec_from_file_location(
    "nvs_rules", Path(__file__).resolve().parent / "rules.py")
rulesmod = importlib.util.module_from_spec(_RULES)
# Registered before it runs: a `@dataclass` resolves its annotations through `sys.modules`.
sys.modules["nvs_rules"] = rulesmod
_RULES.loader.exec_module(rulesmod)


def say(line: str = "") -> None:
    print(line)


def git(*args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True,
        encoding="utf-8", errors="replace", check=False,
    ) if not check else _checked(["git", *args])


def _checked(cmd: list[str]) -> subprocess.CompletedProcess:
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                          encoding="utf-8", errors="replace", check=False)
    if proc.returncode != 0:
        raise RuntimeError(f"{' '.join(cmd)} failed ({proc.returncode}):\n"
                           f"{(proc.stdout + proc.stderr).strip()}")
    return proc


# ------------------------------------------------------------------- the wrap file


class Section:
    __slots__ = ("kind", "arg", "body", "line", "added", "targets")

    def __init__(self, kind: str, arg: str, body: str, line: int):
        self.kind, self.arg, self.body, self.line = kind, arg, body, line
        #: Paths `wrap()` added to a `## commit:` because this wrap wrote them and no section
        #: named them. Reported rather than applied silently; see `written_paths`.
        self.added: list[str] = []
        #: A `## playbook:` section's bullets and the file each is written to; see
        #: `playbook_targets`.
        self.targets: list[tuple[Path, str]] | None = None


def parse_wrap(text: str) -> tuple[list[Section], list[str]]:
    """`## kind: arg` headings and their bodies, in file order.

    The handoff body is markdown containing its own `## ` headings, so the parser has to stop
    treating `## ` as a directive once it is inside `## handoff` -- otherwise the handoff's own
    `## State` would read as an unknown instruction. It resumes at the next *known* directive."""
    known = ("plan", "plan-edit", "milestone", "playbook", "handoff", "commit", "status")
    lines = text.split("\n")
    sections: list[Section] = []
    errors: list[str] = []
    cur: Section | None = None
    buf: list[str] = []

    def flush() -> None:
        nonlocal cur, buf
        if cur is not None:
            cur.body = "\n".join(buf).strip("\n")
            sections.append(cur)
        cur, buf = None, []

    for i, raw in enumerate(lines, 1):
        m = re.match(r"^##\s+([A-Za-z][A-Za-z-]*)\s*:?\s*(.*)$", raw)
        directive = m and m.group(1).lower() in known
        # Inside a `## handoff` body, only a *new known directive* ends it. `## State` does not.
        if directive and cur is not None and cur.kind == "handoff":
            if m.group(1).lower() == "handoff":
                errors.append(f"line {i}: a second `## handoff` section")
        if directive:
            flush()
            cur = Section(m.group(1).lower(), m.group(2).strip(), "", i)
            continue
        if cur is None:
            if raw.strip():
                errors.append(f"line {i}: text before the first `## ` directive: {raw.strip()[:60]!r}")
            continue
        buf.append(raw)
    flush()

    for s in sections:
        if s.kind in ("plan", "plan-edit", "milestone", "playbook", "commit") and not s.arg:
            errors.append(f"line {s.line}: `## {s.kind}:` needs an argument")
        if s.kind in ("handoff", "status") and s.arg:
            errors.append(f"line {s.line}: `## {s.kind}` takes no argument, got {s.arg!r}")
        if not s.body.strip():
            errors.append(f"line {s.line}: `## {s.kind}` has an empty body")
    return sections, errors


# ------------------------------------------------------------------------ validation


def normalize(text: str) -> str:
    """A field's text as `plan.py` stores it: one paragraph, single-spaced.

    A status field is hard-wrapped on disk and joined back into one line by `find_fields`, so a
    fragment quoted out of a session's own context has to be compared the same way -- otherwise a
    quote that is correct to the word fails to match because the author's line breaks fell
    somewhere else."""
    return " ".join(text.split())


def parse_edits(body: str) -> tuple[list[tuple[str, str]], list[str]]:
    """`--- old` / `--- new` fragment pairs out of a `## plan-edit:` body, plus what was malformed.

    Deliberately the same shape as an editor's find-and-replace, because that is the operation:
    the field stays where it is and one run of words inside it moves."""
    pairs: list[tuple[str, str]] = []
    errors: list[str] = []
    slot: str | None = None
    buf: dict[str, list[str]] = {"old": [], "new": []}

    def flush() -> None:
        if buf["old"] or buf["new"]:
            pairs.append((normalize("\n".join(buf["old"])), normalize("\n".join(buf["new"]))))
        buf["old"], buf["new"] = [], []

    for raw in body.split("\n"):
        m = re.match(r"^---\s*(old|new)\s*$", raw.strip(), flags=re.I)
        if m:
            nxt = m.group(1).lower()
            if nxt == "old":
                flush()
            elif not buf["old"]:
                errors.append("a `--- new` fragment with no `--- old` in front of it")
            slot = nxt
            continue
        if slot is None:
            if raw.strip():
                errors.append(f"text before the first `--- old`: {raw.strip()[:60]!r}")
            continue
        buf[slot].append(raw)
    flush()

    if not pairs:
        errors.append("no `--- old` / `--- new` fragment pair")
    for old, _new in pairs:
        if not old:
            errors.append("an empty `--- old` fragment -- it must quote what is there now")
    return pairs, errors


#: A `## plan:` section replaces a field whole. Past this size a hand-typed replacement is a
#: RETYPE -- the author re-emitted a field to change a sentence in it -- and the overlap test
#: below is what tells the two apart.
RETYPE_BYTES = 1_500

#: How much of a replacement may already be on disk, word for word, before it is a retype rather
#: than a rewrite. Measured over one 21-session run: `Open now` grew 4,106 -> 43,418 B and was
#: re-emitted whole ten times, 160,388 B of wrap payload in all, 45% of everything those sessions
#: wrote into a wrap file. A 33 KB one cost a single Write 189 seconds. Nothing about that is a
#: decision being made; it is a paragraph being carried across a copy, which is also where a
#: paragraph gets silently dropped.
RETYPE_OVERLAP = 0.70

#: ...unless the replacement is substantially SHORTER than what it replaces, which is a trim and
#: is the one edit that is legitimately all-verbatim: cutting a field down re-emits only the part
#: that survives, and expressing that as fragments would mean one `--- old` per deleted paragraph.
#: A retype comes back the same size or bigger, which is what this separates it by.
RETYPE_SHRINK = 0.60


def verbatim_overlap(new: str, old: str) -> float:
    """The fraction of `new`'s 8-word runs that appear verbatim in `old`.

    Not a similarity score -- the question is one-directional and specific: *how much of what
    you just typed was already there?* Cheap (two set builds) where a real diff over a 43 KB
    field is not, and it does not confuse two documents that merely share a vocabulary, because
    an 8-word run repeats by accident about never."""
    wn, wo = new.split(), old.split()
    if len(wn) < 8:
        return 0.0
    gn = {tuple(wn[i:i + 8]) for i in range(len(wn) - 7)}
    go = {tuple(wo[i:i + 8]) for i in range(len(wo) - 7)}
    return len(gn & go) / len(gn)


def nbytes(s: str) -> int:
    """A field's size the way the plan's aim and ceiling state it: UTF-8 bytes, not characters."""
    return len(s.encode("utf-8"))


def growth_refusal(kind: str, field: str, before: str, after: str, ceiling: int) -> str | None:
    """The one size refusal, and it is a gate on growth rather than on size.

    `Open now` reached 52 KB -- 46% of the orientation pack -- under an advisory note that fired
    at 4x the aim and was ignored for dozens of sessions, because a note at the end of a session
    is read by nobody. doc-style.md § *Length targets* records why a size gate is not the
    answer either: five and ten iterations a session shaving prose to clear a tripwire. This
    is neither. An edit is refused only when it leaves the field both over the ceiling *and*
    bigger than it was, so a field that shrinks or holds its size is always taken and there is
    never prose to shave: the cost of adding a sentence to a full field is dropping one, which
    is what "overwrite a field in place" meant all along. `--check` and `--template` print the
    headroom ahead of the wrap, so a session that reads either never meets this at all."""
    was, now = nbytes(before), nbytes(after)
    if now <= was or now <= ceiling:
        return None
    cut = now - max(ceiling, was)
    how = ("a `--- old` quoting the stale sentence with an empty `--- new` drops it"
           if kind == "plan-edit" else "send the replacement shorter")
    return (f"`## {kind}: {field}` -- leaves the field at {now:,} B, +{now - was} B "
            f"and past its {ceiling:,} B ceiling. A field is status and is overwritten, not appended "
            f"to. Cut at least {cut} B of it in this same section ({how}), or take the new text "
            f"where it belongs -- a per-file gap to that crate's module doc `# Known gaps`, a trap to "
            f"`## playbook:`, what landed to the commit body. An edit that leaves the field no bigger "
            f"than it is now is always taken.")


def plan_fields() -> list[tuple[str, int, int, str]]:
    _text, lines = planmod.load()
    return planmod.find_fields(lines)


def rel_path(path: Path) -> str:
    """Repo-relative and forward-slashed -- the spelling a `## commit:` pathspec uses."""
    return path.relative_to(ROOT).as_posix()


def written_paths(sections: list[Section]) -> list[str]:
    """The tracked files this wrap's doc sections write, in application order, deduplicated.

    Measured over one 19-session run, **9 sessions closed with a hand-rolled `git add
    docs/agent/handoff.md docs/agent/playbook.md docs/implementation-plan.md && git commit`**  # check-links:retired
    after this tool had already written all three -- about two and a half calls each, at the
    point in a session where a call is most expensive, and exactly the hand-rolled git the wrap
    exists to remove. The cause is a chicken-and-egg that is not real: the docs look like they
    cannot be committed by the wrap that writes them, when `ORDER` puts every doc section ahead
    of every commit precisely so they can be.

    So the wrap answers it itself. Anything here that no `## commit:` names is appended to the
    last one, and reported.

    `## status` is not in this list: `.loop/` is gitignored, so nothing it writes is committable.
    """
    out: list[str] = []
    for s in [x for kind in ORDER for x in sections if x.kind == kind]:
        if s.kind in ("plan", "plan-edit"):
            out.append(rel_path(PLAN))
        elif s.kind == "milestone":
            entry = planmod.resolve(s.arg)
            if entry is not None:
                out.append(rel_path(entry["path"]))
        elif s.kind == "playbook":
            out += [rel_path(p) for p, _body in playbook_targets(sections, s)]
        elif s.kind == "handoff":
            out.append(rel_path(HANDOFF))
    return list(dict.fromkeys(out))


def covers(pathspec: str, path: str) -> bool:
    """Does one `## commit:` pathspec carry this file? Git's rule: the path itself, a directory
    above it, or a glob matching it."""
    spec = pathspec.replace("\\", "/").rstrip("/")
    return path == spec or path.startswith(spec + "/") or fnmatch.fnmatch(path, spec)


#: Files no session writes by hand and every session can leave dirty: `bun nv verify` regenerates
#: them in place, on purpose, so the tree follows the registry -- and `fuzz/`'s lock follows the
#: workspace's dependencies -- without a session remembering to.
#: They are derived from what the session's own commits changed, so they belong to this session
#: whatever it named -- and because nothing in the wrap *writes* them, `written_paths` cannot see
#: them and the sweep below would leave them for a hand-rolled `git add` that never comes.
#:
#: This is the one leak that survived `uncommitted_writes`: a session adding a `Core` member ran
#: the verification, which rewrote `docs/novis.md`, and ended with the file dirty. The next session
#: inherited it, and an INTERRUPTED session left it behind for good.
GENERATED = ("docs/novis.md", "fuzz/Cargo.lock")


def dirty_generated() -> list[str]:
    """Which of `GENERATED` the tree has actually changed, in `git`'s own spelling."""
    out = _checked(["git", "status", "--porcelain", "--", *GENERATED]).stdout
    return [ln[3:].strip() for ln in out.split("\n") if ln.strip()]


def refresh_generated(dry: bool) -> str:
    """Regenerate `docs/novis.md` when the tree has left it stale; return a refusal if it cannot.

    `dirty_generated` sweeps the file only after something else has rewritten it, which covers
    the session whose `bun nv verify` ran after its last edit. The other order is the workflow's own:
    step 3 verifies and step 4 writes docs, so a step-4 edit to `docs/spec/02-php-migration.md`
    or to a chapter under `docs/reference/` leaves the reference stale *and clean*. Nothing in
    the session sees that. The driver's acceptance sweep does, after the session is gone, and a
    run that stops there needs a person; a quarter of a second here is what that costs instead.

    A tree with no debug binary cannot answer the question at all, and this refuses rather than
    guessing -- the wrap writes nothing, and `bun nv verify` is the one command that clears it."""
    def run(*args: str) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, str(ROOT / "tools" / "reference.py"), *args],
                              cwd=ROOT, capture_output=True, text=True,
                              encoding="utf-8", errors="replace")
    checked = run("--check")
    if checked.returncode == 0:
        return ""
    said = ((checked.stdout + checked.stderr).strip().split("\n") or [""])[-1]
    if "stale" not in said:
        return f"`python tools/reference.py --check` cannot run: {said}"
    say(f"session.py: docs/novis.md is stale -- "
        f"{'would regenerate' if dry else 'regenerating'} it from the binary")
    if not dry:
        run("--no-examples")
    return ""


def uncommitted_writes(sections: list[Section], extra: list[str] = ()) -> list[str]:
    """What this wrap writes -- or `bun nv verify` wrote under it, or `retire_expired` changed --
    that no `## commit:` carries."""
    specs = [spec for s in sections if s.kind == "commit" for spec in s.arg.split()]
    owed = list(written_paths(sections)) + dirty_generated() + list(extra)
    return [p for p in owed if not any(covers(spec, p) for spec in specs)]


def retire_expired(dry: bool) -> list[str]:
    """Delete every bullet whose `[until:]` condition holds, and name the files that changed.

    Every wrap does this, before anything is staged, because the trailer exists so that no reader
    decides an expiry twice: `playbook.py --retire` is the one write that script makes, and calling
    it here is what turns a declared condition into a deletion without a session remembering to.
    It refuses nothing -- a bullet that declares nothing is `--check`'s finding, and `run_retire`
    says so and deletes nothing. The changed files -- the playbook, and every goal manifest whose
    `playbook` list named a retired bullet alone, which `playbookmod.retire` prunes so the floor's
    `chain.py --check` never meets a selector that reaches nothing -- join the last `## commit:`
    like any other doc the wrap wrote, so `git log` names what expired in the same commit that
    closes the session."""
    expired, _owed, bad, _rows = playbookmod.expiry_report()
    if bad or not expired:
        return []
    files = sorted({e["file"] for e in expired})
    say(f"session.py: {len(expired)} bullet(s) whose retirement condition holds -- "
        f"{'would retire' if dry else 'retiring'} them from {', '.join(files)}")
    return playbookmod.retire(expired, dry)


def validate(sections: list[Section]) -> list[str]:
    """Everything that could refuse, refusing here -- before a single byte is written."""
    errors: list[str] = []
    names = {n.lower(): n for n, _a, _b, _t in plan_fields()}
    # The plan sections are checked against a *simulated* field, advanced in apply order, so a
    # second `## plan-edit:` on the same field quotes the text the first one will have left --
    # which is the text its author was looking at -- rather than the text on disk right now.
    working = {n.lower(): t for n, _a, _b, t in plan_fields()}
    ceiling = planmod.field_ceiling()

    for s in [x for kind in ORDER for x in sections if x.kind == kind]:
        if SIDE and s.kind in ("plan", "plan-edit", "milestone"):
            # The plan's status block and the milestone files are the chain run's state, and a
            # side branch that rewrote them would overwrite main's when it lands.
            errors.append(f"`## {s.kind}: {s.arg}` -- a side run does not write the plan; say "
                          f"what it changed in the handoff, and the landing commit carries it")
            continue
        if s.kind == "plan":
            if s.arg.lower() not in names:
                errors.append(
                    f"`## plan: {s.arg}` -- no such field, and this tool does not add one. "
                    f"The block has: {', '.join(names.values())}")
                continue
            new = normalize(s.body)
            old = working[s.arg.lower()]
            share = verbatim_overlap(new, old)
            if (len(new) > RETYPE_BYTES and share >= RETYPE_OVERLAP
                    and len(new) > len(old) * RETYPE_SHRINK):
                errors.append(
                    f"`## plan: {s.arg}` -- {len(new):,} B, and {share * 100:.0f}% of it is already "
                    f"on disk word for word. That is a retype, not a rewrite: use "
                    f"`## plan-edit: {s.arg}` with `--- old` / `--- new` fragments and send only "
                    f"the sentence that moved. (A real rewrite overlaps less than "
                    f"{RETYPE_OVERLAP * 100:.0f}% and is taken as it stands.)")
                continue
            grown = growth_refusal("plan", s.arg, old, new, ceiling)
            if grown:
                errors.append(grown)
                continue
            working[s.arg.lower()] = new
        elif s.kind == "plan-edit":
            if s.arg.lower() not in names:
                errors.append(
                    f"`## plan-edit: {s.arg}` -- no such field, and this tool does not add one. "
                    f"The block has: {', '.join(names.values())}")
                continue
            pairs, bad = parse_edits(s.body)
            errors += [f"`## plan-edit: {s.arg}` -- {b}" for b in bad]
            text = working[s.arg.lower()]
            for old, new in pairs:
                if not old:
                    continue
                hits = text.count(old)
                if hits != 1:
                    where = "is not in that field" if hits == 0 else f"appears {hits} times in it"
                    errors.append(
                        f"`## plan-edit: {s.arg}` -- the `--- old` fragment {where}, so nothing "
                        f"was changed. Quote a longer run, exactly as the field reads (it is one "
                        f"paragraph, single-spaced; `python tools/plan.py --get {s.arg!r}` prints "
                        f"it): {old[:70]!r}")
                    continue
                text = text.replace(old, new, 1)
            grown = growth_refusal("plan-edit", s.arg, working[s.arg.lower()], text, ceiling)
            if grown:
                errors.append(grown)
                continue
            working[s.arg.lower()] = text
        elif s.kind == "milestone":
            entry = planmod.resolve(s.arg)
            if entry is None:
                ids = ", ".join(m["id"] for m in planmod.milestones())
                errors.append(
                    f"`## milestone: {s.arg}` -- the plan's table lists no such milestone, and "
                    f"this tool does not add one. It has: {ids}")
            elif not entry["path"].exists():
                errors.append(f"`## milestone: {s.arg}` -- {entry['rel']} does not exist")
        elif s.kind == "playbook":
            if playbookmod.section_dir(s.arg) is None:
                errors.append(f"`## playbook: {s.arg}` -- no such section. The playbook has: "
                              f"{', '.join(playbook_headings())}")
            if not s.body.lstrip().startswith("-"):
                errors.append(f"`## playbook: {s.arg}` -- a playbook entry is a `- ` bullet")
            else:
                # A section may carry more than one bullet, so every one of them is asked
                # separately: a trailer is a property of a bullet, and checking the section as a
                # whole reads only the last one's, which lets a malformed trailer in front of a
                # good one land and fail `playbook.py --check` in the next session's floor.
                found = playbookmod.blocks(s.body)
                if not found:
                    errors.append(f"`## playbook: {s.arg}` -- a bullet starts with `- ` at column "
                                  f"0; this one is indented, and the file's own parsers skip it")
                for b in found:
                    which = f"`## playbook: {s.arg}` -- {b['lead'].strip()!r}"
                    if playbookmod.wrapped(b["body"]):
                        errors.append(
                            f"{which} ends with a trailer broken across two lines. Its argument "
                            f"would hold a newline no path, needle, name or date can, so nothing "
                            f"ever retires the bullet. Keep the whole `[until: ...]` on one line.")
                    elif playbookmod.declaration(b["body"]) is None:
                        errors.append(
                            f"{which} declares nothing that retires it. End it with "
                            f"`[until: <kind> <arg>]`; the five kinds are in tools/playbook.py's "
                            f"module doc.")
                    else:
                        errors += reviewed_refusals(which, b["body"])
                    if not playbookmod.anchors(b["body"]):
                        errors.append(
                            f"{which} names no file in the tree. A trap is about a file: name it in "
                            f"backticks, as its path from the repository root (`tools/session.py`, "
                            f"`crates/nvs-ir/src/lib.rs`, `Cargo.toml`). A rule every agent needs "
                            f"whatever it edits is not a trap: it belongs in "
                            f"docs/agent/commands.md or docs/agent/conventions.md.")
                    weight = len(b["body"].strip().encode("utf-8"))
                    if weight > PLAYBOOK_BULLET_MAX:
                        errors.append(
                            f"{which} is {weight} B, past the {PLAYBOOK_BULLET_MAX} B a bullet may "
                            f"weigh. A bullet is the trap, why, and what to do instead -- three "
                            f"sentences, docs/agent/conventions.md § *A playbook bullet*. The "
                            f"session's story (which stage, what was tried first) is git log's.")
                for sel in playbook_collisions(s.arg, s.body):
                    errors.append(
                        f"`## playbook: {s.arg}` -- appending this bullet leaves "
                        f"{sel!r} reachable by no selector, so `orient.py` can no longer hand "
                        f"that trap to a goal that names it. Reword this bullet's lead-in: it "
                        f"is the first sentence in bold, and it has to differ from the one it "
                        f"collides with by more than its tail."
                    )
        elif s.kind == "status":
            first = s.body.strip().split("\n")[0]
            if not first.startswith(STATUS_WORDS):
                errors.append(f"`## status` -- must start with one of {'/'.join(STATUS_WORDS)}, "
                              f"got {first[:40]!r}")
            if len(s.body.strip().split("\n")) > 1:
                errors.append("`## status` -- one line only")
            if first.startswith("DONE"):
                errors.extend(named_test_findings())
        elif s.kind == "commit":
            errors.extend(validate_commit(s))
        elif s.kind == "handoff":
            errors.extend(validate_handoff(s.body))

    # A wrap that writes the docs and commits nothing leaves step 5 owing exactly the files it
    # just changed. There is nothing to guess here -- but inventing a commit subject for them
    # would be this tool judging content, which it does not do -- so it refuses and names them.
    writes = written_paths(sections)
    if writes and not any(s.kind == "commit" for s in sections):
        errors.append(
            f"this wrap writes {', '.join(writes)} and has no `## commit:` section, so step 5 "
            f"would end with {'them' if len(writes) > 1 else 'it'} dirty. Add "
            f"`## commit: {' '.join(writes)}` -- every doc section is applied before any commit "
            f"is staged, so one wrap does both.")

    errors += body_links(sections) + body_citations(sections) + body_goal_numbers(sections)
    broke, _found = link_findings()
    if broke:
        shown = "; ".join(broke[:8])
        more = (f"; and {len(broke) - 8} more -- `python tools/check-links.py` lists them all"
                if len(broke) > 8 else "")
        errors.append(
            f"{len(broke)} link(s) resolved at HEAD and do not resolve now, so they are this "
            f"session's: {shown}{more}. Most often that is a file renamed under the citations of "
            f"it, which is how the four this gate was added for got there. Nothing else catches it "
            f"before the push -- `check-links.py` is CI's `docs` job, and `bun nv verify` does not "
            f"run it, so a green verify says nothing here. "
            f"A link already dead at the commit this session opened on is not counted: that one is "
            f"not yours. A slice you committed by hand earlier in this session is still yours.")
    errors += rulebook_findings() + record_findings() + migration_findings() + manifest_findings()
    return errors


def reviewed_refusals(which: str, body: str) -> list[str]:
    """Why a new bullet may not end with `[until: reviewed <date>]`, if it may not.

    A `reviewed` trailer is the one kind nothing retires: it only asks a reader to look again. So
    a new bullet takes it only when no mechanical kind fits. A bullet that names a path in the
    tree has one: its trap ends when the path, or a word in it, goes. And a date is the day
    somebody read the bullet, so a date after today records no reading."""
    kind, arg = playbookmod.declaration(body)
    if kind != "reviewed":
        return []
    out = []
    try:
        if date.fromisoformat(arg) > date.today():
            out.append(f"{which} is dated {arg}, which is after today. A `reviewed` date is the day "
                       f"the bullet was read: write today's date.")
    except ValueError:
        out.append(f"{which} -- `{arg}` is not a YYYY-MM-DD date.")
    named = playbookmod.anchors(body)
    if named:
        out.append(
            f"{which} names `{named[0]}` and ends with `[until: reviewed ...]`. A trap about a path "
            f"ends when that path changes, so declare that instead: `[until: gone {named[0]}:<a "
            f"word the trap depends on>]`, `[until: exists <path>]` for a fix that is not there "
            f"yet, or `[until: test <fn>]` for a hole a test will close.")
    return out


def validate_commit(s: Section) -> list[str]:
    errors = []
    subject = s.body.strip().split("\n")[0]
    if not re.match(r"^(feat|fix|docs|test|perf|refactor|chore|build|ci)"
                    r"(\([a-z0-9-]+\))?: .+", subject):
        errors.append(f"`## commit:` line {s.line} -- subject is not "
                      f"`type(scope): subject`: {subject[:60]!r}")
    if len(subject) > SUBJECT_MAX:
        # Quote it. This one rule produced EVERY wrap refusal of one 21-session run -- 13 of them
        # -- and the message said only which line, so a session with two commits in the file
        # shortened the wrong subject, got the identical refusal back, and spent a third call on
        # `sed -n` to find out which one it had missed. The subject is the whole answer.
        errors.append(
            f"`## commit:` line {s.line} -- subject is {len(subject)} chars, "
            f"{len(subject) - SUBJECT_MAX} over the {SUBJECT_MAX} limit: {subject!r}")
    for path in s.arg.split():
        target = ROOT / path
        if not target.exists() and "*" not in path:
            errors.append(f"`## commit:` line {s.line} -- no such path: {path}")
    return errors


#: An anchor written without its directory -- `ctx.rs:2285` -- which reads fine and resolves to
#: nothing: `orient.ANCHOR_RE` needs the repo-rooted path, so the next pack silently skips it.
BARE_ANCHOR_RE = re.compile(r"(?<![\w/.-])([\w-]+\.(?:rs|py|md|toml)):(\d+)\b")


def validate_handoff(body: str) -> list[str]:
    errors = []
    for required in HANDOFF_REQUIRED:
        if not re.search(rf"^{re.escape(required)}\b", body, flags=re.M):
            errors.append(f"`## handoff` -- missing the required `{required}` heading")
    errors.extend(validate_anchors(body))
    return errors


def next_group_items(body: str) -> list[tuple[int, str]]:
    """The `- [ ]` checklist items under `## Next group`, (ordinal, text with continuation lines)."""
    span = heading_index(body, "Next group")
    if span is None:
        return []
    lines = body.split("\n")[span[0] + 1 : span[1]]
    items: list[list[str]] = []
    for ln in lines:
        if re.match(r"^\s*- \[", ln):
            items.append([ln])
        elif items:
            items[-1].append(ln)
    return [(n + 1, "\n".join(blk)) for n, blk in enumerate(items)]


def validate_anchors(body: str) -> list[str]:
    """Every unticked item must carry an anchor the next session's driver will actually expand.

    The gate this replaced accepted one `file:NN` anywhere in the handoff, and measured over one
    run the handoffs it passed carried 1 anchor against 10 files named -- in `## State`, or as a
    bare `ctx.rs:NN` -- while `discovery` was 14% of the run's context: every session re-derived
    with `grep` what the previous one had open. `orient.run_anchors` inlines the code at each
    anchor into the pack for nothing, but only from the current item and only when the path is
    repo-rooted, so that is exactly what is checked, per item, and nothing looser. A refusal
    here costs one `--locate` call now; the anchor's absence costs the next session ten."""
    errors = []
    for n, item in next_group_items(body):
        if re.match(r"^\s*- \[[xX]\]", item):
            continue
        head = re.sub(r"^\s*- \[.\]\s*", "", item.split("\n")[0])
        claim = re.sub(r"\s+", " ", re.sub(r"[`*_]", "", head)).strip()[:60]
        for name, line in BARE_ANCHOR_RE.findall(item):
            errors.append(
                f"`## handoff` -- `## Next group` item {n} ({claim!r}) anchors `{name}:{line}` "
                f"without its directory, which orient.py cannot expand. Write it repo-rooted, "
                f"as `crates/.../{name}:{line}`.")
        if not orient.ANCHOR_RE.search(item):
            errors.append(
                f"`## handoff` -- `## Next group` item {n} ({claim!r}) carries no repo-rooted "
                f"`file:NN` anchor. They are not optional: orient.py inlines the code at each one "
                f"into the next session's pack, from the item and nowhere else. Resolve them with "
                f"`python tools/peek.py --locate <symbol> ...`.")
    return errors


def heading_index(text: str, wanted: str) -> tuple[int, int] | None:
    """(start, end) line indices of a `## heading` and its body, matched on its words."""
    def norm(t: str) -> str:
        return re.sub(r"\s+", " ", re.sub(r"[`*_#]", "", t)).strip().lower()

    lines = text.split("\n")
    key = norm(wanted)
    heads = [(i, len(m.group(1)), m.group(2)) for i, ln in enumerate(lines)
             if (m := re.match(r"^(#{1,6})\s+(.*)$", ln))]
    for n, (idx, level, title) in enumerate(heads):
        if norm(title) == key:
            end = len(lines)
            for later_idx, later_level, _ in heads[n + 1 :]:
                if later_level <= level:
                    end = later_idx
                    break
            return idx, end
    return None


# -------------------------------------------------------------------------- the link gate

#: What each `check-links.py` finding means, so a refusal says it rather than naming a kind. That
#: gate's own docstring is the home of all four; these are the one-line readings of them.
LINK_WHY = {
    "missing": "nothing is there",
    "case": "the entry on disk is spelled with different case, which resolves on this machine "
            "and 404s on every Linux checkout",
    "absolute": "a markdown file's links are relative to itself, and `/docs/...` is the site root",
    "relative": "a source file's links are absolute from the repository root (`/docs/...`)",
}

#: Where a wrap section's body lands, for the links inside it: a body's links resolve from the file
#: it is written INTO, not from anywhere this tool runs. `milestone` is absent because its
#: destination is one lookup per id, and `commit`/`status` because neither is a rendered file.
#: A playbook bullet lands in a file of its own, one directory below `docs/agent/playbook/`, and
#: every section's directory is at that depth, so the first one stands for all of them.
BODY_HOME = {"handoff": HANDOFF, "plan": PLAN, "plan-edit": PLAN,
             "playbook": playbookmod.PLAYBOOK_DIR / playbookmod.SECTIONS[0][0] / "bullet.md"}


#: Where `loop.py` leaves the commit the running session opened on. Absent outside the loop, and
#: absent inside it whenever the driver could not name a sha, which both read as "use HEAD".
SESSION_BASE = RUNDIR / "session-start.json"


def session_base() -> str:
    """The commit this session opened on, or `"HEAD"` when nothing on disk names one.

    Checked, not trusted. A sha that no longer resolves, or one that is not an ancestor of HEAD --
    a rebase, a reset, a file left behind by an abandoned run -- would silently widen the baseline
    to include commits this session never made, and a gate that blames the wrong session gets
    turned off. Anything that does not check out falls back to HEAD, which is the old behaviour and
    is never wrong in the direction that matters: it under-refuses.
    """
    try:
        base = json.loads(SESSION_BASE.read_text(encoding="utf-8")).get("base", "")
    except (OSError, ValueError, AttributeError):
        return "HEAD"
    if not isinstance(base, str) or not re.fullmatch(r"[0-9a-f]{7,40}", base):
        return "HEAD"
    if git("cat-file", "-e", f"{base}^{{commit}}", check=False).returncode != 0:
        return "HEAD"
    if git("merge-base", "--is-ancestor", base, "HEAD", check=False).returncode != 0:
        return "HEAD"
    return base


def tree_paths(ref: str) -> set[str]:
    """Every path `ref` holds, spelled as git spells it -- the baseline the link gate compares to.

    Empty when there is no such commit to read, which the caller then reads as "every finding is
    new"."""
    done = git("ls-tree", "-r", "--name-only", ref, check=False)
    if done.returncode != 0:
        return set()
    return {line for line in done.stdout.split("\n") if line}


def in_tree(paths: set[str]):
    """A `check-links` resolver answering from a set of repo-relative paths instead of from disk.

    Membership is an exact string compare, so this is case-exact for free: git records the spelling
    a file was added with, which is the thing `Path.exists()` cannot tell you on Windows or macOS.
    """
    def resolve(base: Path, target: str) -> str | None:
        prefix = base.as_posix()[len(ROOT.as_posix()):].strip("/")
        rel = posixpath.normpath(posixpath.join(prefix, target))
        if rel == ".." or rel.startswith("../"):
            return None  # walked out of the repository: not ours to judge, as on disk
        if rel in paths or any(p.startswith(rel + "/") for p in paths):
            return None
        return "missing"
    return resolve


def scanned_files() -> list[Path]:
    """Every file the gate reads: the tracked ones, plus what this session has created.

    `check-links.py` walks `git ls-files` because CI runs it on a clean checkout, where that is
    everything there is. A wrap is the other case -- a file the session wrote is untracked right up
    until the `## commit:` that stages it -- so a gate reading only tracked files would miss exactly
    the citations that have never been read by anything."""
    files = checklinks.tracked_files([])
    done = git("ls-files", "--others", "--exclude-standard", check=False)
    if done.returncode == 0:
        exts = checklinks.DOC_EXTS + checklinks.SOURCE_EXTS + checklinks.MENTION_EXTS
        files += [ROOT / ln for ln in done.stdout.split("\n") if ln and Path(ln).suffix in exts]
    return files


def inherited_links(rel: str, paths: set[str], ref: str) -> set[str]:
    """The link targets already dead in `rel` at `ref`, which are not this session's to answer.

    The branch mirrors `checklinks.check`, and has to: a `.py` file is scanned for bare path
    mentions and never for link syntax, so reading its HEAD text as markdown would find none of
    them, hand back an empty baseline, and charge this session for every stale mention it
    inherited -- the exact over-refusal `link_findings` is written to avoid.
    """
    done = git("show", f"{ref}:{rel}", check=False)
    if done.returncode != 0:
        return set()  # the file is new in this session, so every finding in it is new too
    if Path(rel).suffix in checklinks.MENTION_EXTS:
        return {t for _line, t, _kind in checklinks.dead_mentions(done.stdout)}
    source = Path(rel).suffix in checklinks.SOURCE_EXTS
    return {target for _line, target, _kind in checklinks.findings_in(
        done.stdout, source, (ROOT / rel).parent, in_tree(paths))}


def link_findings() -> tuple[list[str], list[str]]:
    """Dead links in the working tree, split into the ones this session broke and the ones it found.

    **Whole-tree, not the session's diff**, because the way the four findings that put this gate
    here arrived was a *rename*: an ADR moved, and every citation of it went dead in files that
    session never opened. A diff-scoped gate would have missed all four.

    **Only what the session broke refuses**, on the rule `playbook_collisions` already follows: a
    finding that is already in HEAD is CI's to report and a human's to schedule, and refusing a wrap
    over one charges this session for another's -- which in an unattended run means finished,
    verified work left uncommitted behind a link nobody here touched. Those are printed instead.

    **The baseline is the commit the session opened on, and HEAD only when nothing names one.**
    It was HEAD unconditionally, which read every finding a session had already committed as
    inherited -- so a hand-rolled `git commit` before the wrap moved HEAD under the session and
    handed it its own breakage back as somebody else's. That is how a `--record` artifact
    (check-links:written) went dead in `tools/bench-load.py` and reached `main` red, turning CI's
    `docs` job over a link no gate here saw. `session_base` is where the sha comes
    from and what it is checked against; outside the loop there is none, and the old behaviour is
    what an interactive session still gets.

    Reading the baseline costs nothing in the ordinary case: a file with no finding is never asked
    about.
    """
    broke: list[str] = []
    found: list[str] = []
    paths: set[str] | None = None
    ref = session_base()
    for path in sorted(scanned_files()):
        if checklinks.is_generated(path):
            continue
        hits = list(checklinks.check(path))
        if not hits:
            continue
        rel = path.relative_to(ROOT).as_posix()
        if paths is None:
            paths = tree_paths(ref)
        old = inherited_links(rel, paths, ref)
        for lineno, target, kind in hits:
            (found if target in old else broke).append(f"{rel}:{lineno} -> {target} ({kind})")
    return broke, found


#: The two `rules.py` gates, and what a failure of each one means to the session reading it.
RULEBOOK_GATES = (
    (("--check",),
     "a rule does not load, or a `rule:` citation somewhere in the tree names one that is not "
     "there. A renamed or deleted rule breaks its citations in files this session never opened, "
     "which is the same shape as the link gate above and the reason both are whole-tree."),
    (("--render", "--check"),
     "a generated page no longer matches the fragments it is rendered from. `docs/rules/<topic>.md`, "
     "`docs/ground-rules.md` and `docs/divergences.md` are all written by "
     "`python tools/rules.py --render`, and editing a fragment without re-running it commits the "
     "old rendering beside the new rule."),
)

#: The same shape one directory over, and the same argument. A record's field set, heading order and
#: derived counters are `records.py`'s job; that job is CI's `docs` and nothing before the push runs
#: it, so a malformed record reaches `main` under a goal that is closed by the time CI says so.
RECORD_GATES = (
    (("--check",),
     "a decision record's field set, heading order, cross-links or derived counters are wrong -- "
     "most often a `changes:` block with no `modifies:` list, which the checker's own message "
     "spells out. `nv verify` does not run this one either."),
)

#: And one directory further. The migration table is read by two acceptance checks and by nothing a
#: session runs: `reference.py` renders its rows into `docs/novis.md`, and `check-migration.py`
#: audits them against the PHP inventory. The percentage stays the goal's assertion -- `--min` is
#: named by whoever asserts it -- so this gate takes the bare run, which is the structural half.
MIGRATION_GATES = (
    ((),
     "the migration table has a structural error -- a row for a name PHP does not have, an outcome "
     "outside the vocabulary, or a duplicate. The tool's own output is the whole message."),
)


def tree_gate(tree: str, tool: str, gates) -> list[str]:
    """What `tool` refuses over the whole tree, for a session that edited `tree`."""
    done = git("status", "--porcelain", "--", tree, check=False)
    if done.returncode != 0 or not done.stdout.strip():
        return []
    out: list[str] = []
    for args, why in gates:
        ran = subprocess.run(
            [sys.executable, str(ROOT / "tools" / tool), *args],
            cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace",
        )
        if ran.returncode == 0:
            continue
        detail = (ran.stdout + ran.stderr).strip().split("\n")
        shown = "\n      ".join(detail[:12])
        more = f"\n      ... and {len(detail) - 12} more line(s)" if len(detail) > 12 else ""
        out.append(
            f"`{' '.join(('python', f'tools/{tool}', *args))}` fails, and this session edited "
            f"`{tree}/`: {why}\n      {shown}{more}")
    return out


def rulebook_findings() -> list[str]:
    """What `tools/rules.py` refuses, but only for a session that touched `docs/rules/`.

    The rulebook is the home of every rule that is currently true, and the tree cites it about
    18,500 times -- so it is the most-read structure in `docs/` and, until this gate, the only part
    of `docs/` nothing checked. `nv verify` deliberately does not run a docs gate (its own docstring
    says why), and CI's `docs` job is after the push, so a session that renamed a rule learned about
    its 40 dead citations from a red `lint` job rather than from the wrap that wrote them.

    **The trigger is the session's own edit, not the rulebook's state.** `link_findings` can ask
    HEAD whether a finding is inherited because a link resolves per file; a rulebook finding is a
    property of the whole tree and there is nothing per-file to compare. So the conservative
    equivalent is to run these only when this session has changed something under `docs/rules/` --
    including a rename, which is what makes a citation elsewhere go dead. A rulebook already red
    under a session that never opened it is CI's finding and a human's to schedule, exactly as an
    inherited dead link is.
    """
    return tree_gate("docs/rules", "rules.py", RULEBOOK_GATES)


def record_findings() -> list[str]:
    """What `tools/records.py` refuses, but only for a session that wrote a decision record.

    The trigger is the session's own edit, exactly as it is above and for the same reason: a
    record's shape is a property of the whole set and there is nothing per-file to compare, so a
    set already red under a session that never opened `docs/decisions/` is CI's finding and a
    person's to schedule."""
    return tree_gate("docs/decisions", "records.py", RECORD_GATES)


def migration_findings() -> list[str]:
    """What `tools/check-migration.py` refuses, for a session that edited `docs/spec/`.

    Same trigger, same reason. The threshold this table is really gated on lives in the goal's
    acceptance list and nowhere else, so the bare run is what belongs here: it answers whether the
    rows are well formed, which is the half a session can break and then close on."""
    return tree_gate("docs/spec", "check-migration.py", MIGRATION_GATES)


def manifest_findings() -> list[str]:
    """What `chain.py --check` would refuse in the live goal's `[context]` manifest, always.

    The same reading `orient.manifest_findings` gives the driver -- a `playbook` selector that
    reaches no bullet, a `shapes` heading conventions.md does not have -- taken here, before the
    commit, because that check sits on every goal's floor and the driver runs the floor after the
    session is gone: a manifest broken at wrap time is a DONE claim held for a hand. No trigger,
    unlike the three above: the manifest is one file and the playbook one read, so the whole gate
    costs less than a link check, and there is no earlier session to blame it on -- the wrap that
    retires a bullet prunes the lines that named it, so a selector reaching nothing here was
    written or unwritten by this session."""
    goal = SIDE or goalsmod.find(goalsmod.load(), goalsmod.live())
    if goal is None or goal.retired:
        return []
    # Both copies: the goal's own file is what the floor's `chain.py --check` reads, and the
    # installed `loop-goal.toml` is what the pack and the driver's sweep read -- a session edits
    # the latter in place, so the two can disagree. A side goal has one copy, and both name it.
    manifests = list(dict.fromkeys(
        [goal.toml] + ([orient.GOAL_TOML] if orient.GOAL_TOML.is_file() else [])))
    problems: list[str] = []
    for toml in manifests:
        found, _notes = orient.manifest_findings(toml)
        problems += [p for p in found if p not in problems]
    return [f"{p} -- `chain.py --check` is on the floor and halts a DONE claim on this; fix the "
            f"manifest before the wrap, or drop the line" for p in problems]


def named_test_findings() -> list[str]:
    """Every test the live goal's `cargo-named` checks name that is not a `fn` in the tree -- on a
    DONE claim only.

    A `cargo-named` check passes when its `tests` names all appear in cargo's output, and a filter
    that matches no test runs zero of them and exits 0, so the name is the whole check. A goal's
    toml names its tests before they are written, and the release-profile ones sit behind the
    driver's floor gate in every scoped run; a test written under a near miss of the toml's name
    is green for the whole goal and surfaces once, in the sweep that confirms the DONE claim, as a
    hand the run waits for. The names come from `playbook.test_names` -- the same `git grep` its
    `[until: test <name>]` trailer uses -- over both copies of the goal, for the reason
    `manifest_findings` reads both, and a name matches as a substring of a `fn`, because that is
    how the driver reads cargo's output and how cargo's own filter selects: a check naming
    `..._are_refused` is met by `fn ..._are_refused_naming_both`. A CONTINUE is not gated: a test
    not yet written is what the stages ahead of it are for."""
    goal = SIDE or goalsmod.find(goalsmod.load(), goalsmod.live())
    if goal is None or goal.retired:
        return []
    manifests = list(dict.fromkeys(
        [goal.toml] + ([orient.GOAL_TOML] if orient.GOAL_TOML.is_file() else [])))
    on_disk = playbookmod.test_names()
    missing: list[tuple[str, str]] = []
    for toml in manifests:
        try:
            spec = tomllib.loads(toml.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError):
            continue
        for c in spec.get("check", []):
            if c.get("kind") != "cargo-named":
                continue
            for name in c.get("tests", []):
                if (c.get("name", "?"), name) in missing:
                    continue
                if not any(name in fn for fn in on_disk):
                    missing.append((c.get("name", "?"), name))
    return [f"`## status` DONE -- check {label!r} names test `{name}`, and no `fn {name}` is in "
            f"the tree; the driver's sweep runs it, matches nothing and halts the claim. Rename the "
            f"test to what the toml names, or write it" for label, name in missing]


def body_links(sections: list[Section]) -> list[str]:
    """Dead links in the bodies this wrap is about to write -- caught before it writes them.

    One call writes the handoff, the playbook and the plan *and commits them*, so a link written
    into one of those bodies is in `git log` by the time anything reads it. This is the last moment
    it is cheap, and it is the one path the tree gate above cannot see: at this point the body is
    still only in the wrap file."""
    out: list[str] = []
    for s in sections:
        home = BODY_HOME.get(s.kind)
        if s.kind == "milestone":
            entry = planmod.resolve(s.arg)
            home = entry["path"] if entry else None  # validate() reports an unknown id itself
        if home is None:
            continue
        # A `plan-edit` quotes the field as it reads in its `--- old` half. Only `--- new` is text
        # this wrap puts there, and a dead link that an edit *deletes* is not a finding.
        body = "\n".join(new for _old, new in parse_edits(s.body)[0]) \
            if s.kind == "plan-edit" else s.body
        for _line, target, kind in checklinks.findings_in(body, False, home.parent):
            named = f"`## {s.kind}: {s.arg}`" if s.arg else f"`## {s.kind}`"
            out.append(
                f"{named} cites {target!r}, and {LINK_WHY[kind]}. A link in a wrap body resolves "
                f"from {rel_path(home)}, which is where the body lands -- and this wrap writes and "
                f"commits in one call, so nothing reads it before CI's `docs` job does.")
    return out


def body_citations(sections: list[Section]) -> list[str]:
    """`rule:` tokens in the bodies this wrap is about to write that name no rule -- caught before
    it writes them.

    `rulebook_findings` runs `rules.py --check` over the tree, and the tree at that moment does not
    yet hold these bodies: the wrap writes the handoff and the playbook AFTER validation and commits
    them in the same call. So a placeholder id typed into either -- `rule:` followed by a made-up
    topic and slug, as an illustration -- is in `git log` before anything scans it, and the goal's
    `1 floor` rulebook check is red for every session after, while the session that wrote it has
    already reported the check clean. The tree gate cannot see it for the same reason `body_links`
    exists: at this point the body is still only in the wrap file. The rulebook's spelling of a
    citation is `rules.py`'s `CITATION`, so what that check would flag tomorrow is what this refuses
    today, with the line to fix."""
    book = rulesmod.Rulebook()
    if not book.by_id:
        return []  # no rulebook on disk, so nothing to resolve against -- `rules.py` says the same
    out: list[str] = []
    for s in sections:
        if s.kind not in BODY_HOME and s.kind != "milestone":
            continue
        # A `plan-edit` quotes the field as it reads in its `--- old` half; only `--- new` is text
        # this wrap puts there.
        body = "\n".join(new for _old, new in parse_edits(s.body)[0]) \
            if s.kind == "plan-edit" else s.body
        for line_no, line in enumerate(body.split("\n"), 1):
            for rid in rulesmod.CITATION.findall(line):
                if rid in book.by_id:
                    continue
                named = f"`## {s.kind}: {s.arg}`" if s.arg else f"`## {s.kind}`"
                out.append(
                    f"{named} line {line_no} cites `rule:{rid}`, and no rule has that id. `python "
                    f"tools/rules.py --check` resolves every `rule:` token under `docs/`, the "
                    f"handoff and the playbook included, and the goal's `1 floor` runs it -- so a "
                    f"placeholder id written here turns that check red for every later session. "
                    f"Cite a real rule (`python tools/brief.py --where <keyword>` finds one), or "
                    f"describe the token in words.")
    return out


def body_goal_numbers(sections: list[Section]) -> list[str]:
    """A goal named by its NUMBER in a body this wrap is about to write, refused before it lands.

    `chain.py --check` scans the tree for `goal 29` and sits on every goal's floor, so the wrap that
    writes one into the handoff reports a green session and leaves the next one opening on a red
    check it did not cause -- which is how a handoff came to say the generated goals were appended
    at a numbered range, and this gate's own docstring cannot quote that sentence either. The
    reason the rule exists is the reason the gate has to be here: the sentence is true when it is
    written and false the moment anything is inserted in front of that goal, and by then its author
    is gone. AGENTS.md's *The schedule is the chain* is the rule; its spelling, and the three headers
    that are allowed to carry a number because they are the goal naming itself, are `chain.py`'s.

    A `## commit:` body is gated too, and it is the only section here no later scan can reach: a
    message is not a file, so `chain.py --check` never sees it and `git log` keeps the number
    forever."""
    import chain as chainmod  # here, not at the top: it imports `loop`, which no other path needs
    out: list[str] = []
    for s in sections:
        if s.kind not in BODY_HOME and s.kind not in ("milestone", "commit"):
            continue
        # As in `body_citations`: only a `plan-edit`'s `--- new` half is text this wrap puts there.
        body = "\n".join(new for _old, new in parse_edits(s.body)[0]) \
            if s.kind == "plan-edit" else s.body
        for line_no, line in enumerate(body.split("\n"), 1):
            if (chainmod.H1_RE.match(line) or chainmod.TOML_HEAD_RE.match(line)
                    or chainmod.HANDOFF_RE.match(line)):
                continue
            for m in chainmod.NUMBER_CITE_RE.finditer(line):
                named = f"`## {s.kind}: {s.arg}`" if s.arg else f"`## {s.kind}`"
                out.append(
                    f"{named} line {line_no} writes `{m.group(0)}`, naming a goal by its number. Say "
                    f"the slug -- goal `dossier`, never `goal 71` -- because a number is a position "
                    f"and every insert in front of it moves the position without touching the "
                    f"sentence. `python tools/chain.py --check` refuses this over the whole tree and "
                    f"is on the goal's `1 floor`, so a number written here turns that check red for "
                    f"every later session. A position beside a total (`29 of 43`) is not this.")
    return out


# ---------------------------------------------------------------------------- apply


def apply_plan(s: Section, dry: bool) -> str:
    text, lines = planmod.load()
    fields = planmod.find_fields(lines)
    target = next(f for f in fields if f[0].lower() == s.arg.lower())
    name, start, end, old = target
    new = " ".join(s.body.split())
    if dry:
        return f"plan: {name}  {nbytes(old)} -> {nbytes(new)} bytes"
    rewritten = lines[:start] + ["> " + ln for ln in planmod.render(name, new)] + lines[end:]
    PLAN.write_text("\n".join(rewritten), encoding="utf-8", newline="")
    return f"plan: {name}  {nbytes(old)} -> {nbytes(new)} bytes"


def apply_plan_edit(s: Section, dry: bool) -> str:
    """One field, with the fragments `## plan-edit:` names replaced and nothing else touched.

    `validate()` has already proved every `--- old` matches exactly once, against the field as
    the sections ahead of this one will have left it, so the replacements here cannot miss."""
    text, lines = planmod.load()
    fields = planmod.find_fields(lines)
    name, start, end, old = next(f for f in fields if f[0].lower() == s.arg.lower())
    pairs, _ = parse_edits(s.body)
    new = old
    for was, now in pairs:
        new = new.replace(was, now, 1)
    ceiling = planmod.field_ceiling(text)
    was, now = nbytes(old), nbytes(new)
    note = f"plan-edit: {name}  {len(pairs)} fragment(s), {was} -> {now} bytes"
    # Only when the field GREW, and only when the growth `validate` refuses is now close. A
    # note that fires on every edit is a note every session learns to skip; this one says the
    # next growing edit will be refused, which is the one thing worth knowing ahead of it.
    headroom = ceiling - now
    if now > was and headroom < ceiling // 4:
        note += (f"  (+{now - was} B; {max(headroom, 0)} B left under the {ceiling} B"
                 f" ceiling -- past it a growing edit is refused, so the next one replaces a"
                 f" sentence rather than adding one)")
    if dry:
        return note
    rewritten = lines[:start] + ["> " + ln for ln in planmod.render(name, new)] + lines[end:]
    PLAN.write_text("\n".join(rewritten), encoding="utf-8", newline="")
    return note


def apply_milestone(s: Section, dry: bool) -> str:
    entry = planmod.resolve(s.arg)
    assert entry is not None  # validate() proved it
    old = len(planmod.body_of(entry).encode("utf-8"))
    new = len(s.body.strip("\n").encode("utf-8"))
    note = f"milestone: {entry['id']} in {entry['rel']}, {old} -> {new} bytes"
    if dry:
        return note
    planmod.write_body(entry, s.body)
    if not planmod.verify_paragraph(entry):
        note += "  !! no `**Verify:**` paragraph -- nothing can call this milestone done"
    return note


def playbook_with(text: str, heading: str, body: str) -> str | None:
    """`text` with this bullet appended under `heading`; None if there is no such heading.

    The one home of *where* an appended bullet lands. `apply_playbook` writes what this returns
    and `playbook_collisions` asks what it would mean, so the check and the write cannot end up
    describing two different files."""
    span = heading_index(text, heading)
    if span is None:
        return None
    lines = text.split("\n")
    # Append at the end of the section's own bullets, before whatever heading follows, keeping
    # exactly one blank line ahead of that heading. This file is append-mostly by decision
    # (AGENTS.md § *Keep each slice small*); nothing here rewrites a bullet that is already there.
    at = span[1]
    while at > 0 and not lines[at - 1].strip():
        at -= 1
    lines[at:at] = body.rstrip().split("\n")
    return "\n".join(lines)


def playbook_collisions(heading: str, body: str) -> list[str]:
    """Selectors this bullet would make unreachable -- reported as a difference, not a total.

    A lead-in two bullets share makes *both* of them unfetchable, and `orient.py` fetches a trap
    by exactly that string, so an appended bullet can silently cost the session a trap it already
    had. This is the moment that is cheap to catch: the collision exists because of the wording
    in front of us, and rewording it now costs a line.

    Only what this append would *introduce* is reported. A selector already unreachable in the
    committed file is `playbook.py --check`'s finding to raise, and blocking this wrap over it
    would charge one session for another session's collision."""
    text = playbookmod.read()
    after = playbook_with(text, heading, body)
    if after is None:
        return []  # validate() reports the missing heading itself

    def unreachable(t: str) -> set[str]:
        out = set()
        for b in playbookmod.all_bullets(t):
            hits, complaint = orient.slice_bullets(t, b["selector"])
            if complaint or len(hits) != 1:
                out.add(b["selector"])
        return out

    return sorted(unreachable(after) - unreachable(text))


def playbook_targets(sections: list[Section], s: Section) -> list[tuple[Path, str]]:
    """Each bullet of a `## playbook:` section and the new file it is written to.

    Worked out once for every playbook section in the wrap, in order, and kept on the section: the
    commit paths are listed before anything is written, and the files written must be those."""
    if s.targets is None:
        taken: set[Path] = set()
        for x in sections:
            if x.kind != "playbook":
                continue
            x.targets = []
            for b in playbookmod.blocks(x.body):
                path = playbookmod.new_fragment(x.arg, b["body"], frozenset(taken))
                if path is not None:
                    taken.add(path)
                    x.targets.append((path, b["body"].rstrip() + "\n"))
    return s.targets


def apply_playbook(s: Section, dry: bool) -> str:
    targets = playbook_targets([s], s) if s.targets is None else s.targets
    if not dry:
        for path, body in targets:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(body, encoding="utf-8", newline="\n")
    return f"playbook: + {len(targets)} bullet(s) under {s.arg!r}: " + \
        ", ".join(rel_path(p) for p, _body in targets)


def apply_handoff(s: Section, dry: bool) -> str:
    body = s.body.rstrip() + "\n"
    if not body.lstrip().startswith("# "):
        body = "# Handoff\n\n" + body.lstrip()
    n = len(body.split("\n"))
    note = f"handoff: {n} lines"
    if n > HANDOFF_TARGET_LINES + 25:
        note += (f"  (over the ~{HANDOFF_TARGET_LINES}-line target by {n - HANDOFF_TARGET_LINES};"
                 " a target for the author, not a check -- do not spend a turn trimming it)")
    if not dry:
        HANDOFF.write_text(body, encoding="utf-8", newline="\n")
    return note


#: Trailers this project does not keep, by the key they are spelled with. A commit message
#: documents the work and stops -- conventions.md § *A commit message* is the home of that rule.
#: Attribution boilerplate is noise every future reader of `git log` pays for, and several
#: harnesses append it by default, so stripping is not optional politeness: it is the only thing
#: standing between the convention and a tool that never read it.
TRAILER_RE = re.compile(
    r"^\s*(?:co-authored-by|signed-off-by|generated-(?:by|with)|assisted-by|"
    r"authored-by|reviewed-by|on-behalf-of|committed-by|created-by|made-with)\s*:.*$",
    re.I | re.M,
)

#: The same boilerplate in its prose spelling -- `🤖 Generated with [Some Tool](url)` -- which is
#: not a `Key: value` trailer and so slips past the one above. Deliberately narrow: it wants the
#: emoji or a markdown link right there, because "generated with" is an ordinary English phrase
#: a commit body about a code generator has every right to use.
BOILERPLATE_RE = re.compile(
    # Decoration in front of the phrase (an emoji, a bullet) but never a real word, so "the table
    # is generated with a build script" keeps its place. Spelled with an explicit ASCII range
    # rather than `[^\w\n]` so it asks exactly the question the commit-msg hook asks -- the two
    # gates have to agree, and `\w` is Unicode-aware here while the hook's test is not.
    #
    # A backtick before the phrase means the line is QUOTING the banned form, not carrying it, and
    # the commit that introduced this check was itself the first casualty: a body opening a line
    # with "`🤖 Generated with [tool](url)` is the commonest spelling" had that whole line deleted
    # out of it, silently, leaving the sentence after it dangling. Generated boilerplate does not
    # use backticks; prose about generated boilerplate does.
    r"^[^A-Za-z\n`]*(?:generated|created|made)\s+(?:with|by)\s+\[.*$",
    re.I | re.M,
)


def strip_trailers(body: str) -> tuple[str, int]:
    """The message with every attribution trailer removed, and how many went.

    Removed wherever they sit, not only at the foot: a harness that appends one to a body that
    already ends in a paragraph leaves it mid-message often enough. Blank runs left behind are
    collapsed so the result is a message that reads as though the trailer was never written."""
    cleaned, n = TRAILER_RE.subn("", body)
    cleaned, m = BOILERPLATE_RE.subn("", cleaned)
    if not n and not m:
        return body, 0
    cleaned = re.sub(r"\n{3,}", "\n\n", cleaned)
    return cleaned.strip() + "\n", n + m


def apply_commit(s: Section, dry: bool) -> str:
    paths = s.arg.split()
    subject = s.body.strip().split("\n")[0]
    grew = f"  [+{len(s.added)} this wrap wrote: {' '.join(s.added)}]" if s.added else ""
    if dry:
        return f"commit: {subject[:70]}  ({len(paths)} path(s)){grew}"
    TMP.mkdir(exist_ok=True)
    msg = TMP / "session-commit.txt"
    text, stripped = strip_trailers(s.body.rstrip() + "\n")
    msg.write_text(text, encoding="utf-8", newline="\n")
    _checked(["git", "add", "--", *paths])
    staged = _checked(["git", "diff", "--cached", "--name-only", "--", *paths]).stdout.split()
    if not staged:
        return f"commit: SKIPPED, nothing staged for {' '.join(paths)}"
    # Pathspec-limited on purpose: a `## commit:` commits the paths it names and nothing else,
    # so an index left dirty by something outside this wrap -- another session, an aborted
    # command -- cannot be swept into a message that does not describe it. The playbook has that
    # trap under `git commit -a`; this closes it for every commit a session makes.
    _checked(["git", "commit", "-F", str(msg), "--", *paths])
    head = _checked(["git", "log", "-1", "--format=%h %s"]).stdout.strip()
    note = f"  [{stripped} trailer(s) stripped]" if stripped else ""
    return f"commit: {head}  ({len(staged)} file(s)){note}{grew}"


def apply_status(s: Section, dry: bool) -> str:
    line = s.body.strip()
    if not RUNDIR.is_dir():
        return "status: skipped -- no .loop directory (not a loop session)"
    if not dry:
        STATUS.write_text(line, encoding="utf-8", newline="\n")
    return f"status: {line[:80]}"


APPLY = {"plan": apply_plan, "plan-edit": apply_plan_edit, "milestone": apply_milestone,
         "playbook": apply_playbook, "handoff": apply_handoff, "commit": apply_commit,
         "status": apply_status}
#: `plan-edit` after `plan` so a session may do both to one field in one wrap -- replace it, then
#: patch the replacement -- and so `validate()`'s simulation and this agree on what each sees.
ORDER = ["plan", "plan-edit", "milestone", "playbook", "handoff", "commit", "status"]


def wrap(path: Path, dry: bool) -> int:
    if not path.exists():
        say(f"session.py: no such file: {path}")
        return 2
    sections, errors = parse_wrap(path.read_text(encoding="utf-8"))
    if not sections and not errors:
        say(f"session.py: {path} holds no `## ` directive")
        return 2
    errors += validate(sections)
    if errors:
        say(f"session.py: {len(errors)} problem(s) -- NOTHING was written or committed:")
        for e in errors:
            say(f"  - {e}")
        return 1

    # Fixed order, not file order, so the docs are on disk before anything is staged.
    ordered = [s for kind in ORDER for s in sections if s.kind == kind]

    # The safety net behind `written_paths`: a doc this wrap writes that no `## commit:` names
    # joins the last one rather than being left dirty for a hand-rolled `git add`. The last is
    # the right one because a session's commits read oldest-first and its docs commit is the one
    # that closes the session. Explicit is still better -- `--template` pre-fills the section --
    # and when the session was explicit this finds nothing to do.
    commits = [s for s in ordered if s.kind == "commit"]
    # Before `uncommitted_writes` asks what is dirty, so that a reference regenerated here is
    # dirty by the time it asks and joins the last commit with everything else the wrap wrote.
    stop = refresh_generated(dry)
    if stop:
        say("session.py: NOTHING was written or committed:")
        say(f"  - {stop}")
        return 1
    owed = uncommitted_writes(sections, retire_expired(dry))
    if owed and commits:
        commits[-1].arg = " ".join(commits[-1].arg.split() + owed)
        commits[-1].added = owed

    say(f"session.py: {'would apply' if dry else 'applied'} {len(ordered)} section(s)")
    for s in ordered:
        say(f"  {APPLY[s.kind](s, dry)}")
    if not dry:
        # Everything a session would otherwise go and look up, printed here. Measured over one
        # 21-session run, 13 of them closed with a `git log --oneline`/`git status` AFTER this
        # tool had already committed -- a call spent confirming what the tool just did, at the
        # point in a session where a call is most expensive. AGENTS.md § *Session workflow* ends
        # with "After step 5, stop"; this is what makes stopping the cheaper of the two.
        left = _checked(["git", "status", "--short"]).stdout.strip()
        if left:
            say(f"  uncommitted after the wrap ({len(left.split(chr(10)))} path(s)):")
            for ln in left.split("\n")[:12]:
                say(f"    {ln}")
        n = len([s for s in ordered if s.kind == "commit"])
        if n:
            say()
            say(f"== HEAD  (the {n} commit(s) this wrap made, newest first)")
            for ln in _checked(["git", "log", f"-{n}", "--oneline"]).stdout.strip().split("\n"):
                say(f"  {ln}")
            say()
            say("That is step 5. The tree is committed and the handoff is written -- there is")
            say("nothing a `git log`, a `git status` or a second `nv verify` can add. Stop here.")
        record_pack()
    return 0


def record_pack() -> None:
    """Measure the orientation pack this wrap leaves behind, and say if the session grew it.

    The pack is the one cost every session pays on every turn, and it leaks the way an
    append-mostly file always does: measured over 59 sessions it went 59,033 -> 117,617 B at
    +907 B a session, and every one of those bytes was re-billed on all ~98 of a session's calls.
    Nothing noticed, because the only thing that measured it -- `orient.py --audit` -- is read
    when a goal is *written* and never after.

    This is a **report, not a gate**, and the distinction is the whole design. `doc-style.md`
    § *Length targets* records what the gate version cost: a session at the end of its tail,
    context at its peak, shaving prose to clear a tripwire. So this refuses nothing and changes
    no exit code. It writes one line to `.loop/pack-size.jsonl` -- the size, the head and the live
    goal's slug, since a pack that steps across a chain switch was authored rather than accumulated
    -- and, when the session grew the pack past `PACK_NOTE_AT` inside one goal, prints one line
    naming the growth, addressed to whoever writes the next goal, which is the only moment the
    manifest can be narrowed cheaply.

    `python tools/loop-stats.py` turns the log into a slope. A failure here is silent on purpose:
    a missing `orient.py`, an unparseable goal or an unwritable `.loop` must never be the reason
    a wrap that already committed reports failure."""
    try:
        # Captured as **bytes**: the pack is measured in bytes, and decoding it first is both a
        # step backwards and the one way this can fail loudly. Under `text=True` Python decodes
        # the pipe with the console's own codepage, which on this box is cp1252 — the pack's `§`
        # raises `UnicodeDecodeError` inside the reader thread, `proc.stdout` comes back `None`,
        # and the `AttributeError` below escaped the `except` this whole function is wrapped in,
        # failing a wrap that had already committed everything.
        proc = subprocess.run(
            [sys.executable, str(ROOT / "tools" / "orient.py")],
            capture_output=True, cwd=ROOT, timeout=60,
        )
        if proc.returncode != 0 or not proc.stdout:
            return
        size = len(proc.stdout)
    except (OSError, subprocess.SubprocessError):
        return

    previous, previous_goal = None, None
    try:
        if PACK_LOG.exists():
            for line in PACK_LOG.read_text(encoding="utf-8").splitlines():
                if line.strip():
                    entry = json.loads(line)
                    previous, previous_goal = entry.get("bytes"), entry.get("goal")
    except (OSError, ValueError):
        previous = None

    goal = ""
    try:
        goal = goalsmod.live_slug()
        RUNDIR.mkdir(parents=True, exist_ok=True)
        head = _checked(["git", "rev-parse", "--short", "HEAD"]).stdout.strip()
        with PACK_LOG.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps({"bytes": size, "head": head, "goal": goal}) + "\n")
    except (OSError, subprocess.SubprocessError):
        pass

    # A pack measured under another goal was built from another manifest, so the difference is
    # the chain switch's, not this session's.
    if previous is None or (previous_goal and goal and previous_goal != goal):
        return
    grew = size - previous
    if grew < PACK_NOTE_AT:
        return
    say()
    say(f"== PACK  {previous:,} -> {size:,} B  (+{grew:,} this session)")
    say("  Every byte of that is re-billed on every turn of every session after this one.")
    say("  It is not a problem to fix now and NOT something to shave prose against -- it is a")
    say("  number for whoever writes the next goal: `python tools/orient.py --audit` says which")
    say("  section carries it, and a `[context]` entry may name one bullet, not a whole section.")


# --------------------------------------------------------------------------- check


def counts() -> dict[str, int]:
    def nvst(sub: str) -> int:
        d = ROOT / "tests" / sub
        return len(list(d.rglob("*.nvst"))) if d.is_dir() else 0
    adrs = sorted((DOCS / "decisions").glob("[0-9][0-9][0-9][0-9].md"))
    return {
        "conformance": nvst("conformance"),
        "differential": nvst("differential"),
        "adrs": len(adrs),
        "highest_adr": int(adrs[-1].name[:4]) if adrs else 0,
    }


#: How far a `--- old` quote may be widened before it is given up on as un-quotable. Long enough
#: for the sentence a count sits in, short enough that the fragment stays readable in a wrap file.
QUOTE_MAX = 200


def unique_span(body: str, lo: int, hi: int) -> tuple[int, int] | None:
    """Widen `body[lo:hi]` a word at a time until it appears exactly once, or give up.

    A `--- old` fragment that matches twice is refused by `validate`, and "conformance 512" is
    exactly the kind of run of words a status field says more than once. Widening is what a human
    does about that, so it is done here rather than reported.
    """
    while body.count(body[lo:hi]) != 1:
        if hi - lo >= QUOTE_MAX:
            return None
        left = body.rfind(" ", 0, max(lo - 1, 0)) if lo > 0 else -1
        right = body.find(" ", min(hi + 1, len(body))) if hi < len(body) else -1
        if left == -1 and right == -1:
            return None
        if left != -1:
            lo = left + 1
        if right != -1:
            hi = right
    return lo, hi


def stale_edits() -> list[tuple[str, str, str]]:
    """`(field, old, new)` for every count in the plan that disagrees with the tree.

    This is the edit **every** session of a counted goal makes, and it was being derived by hand:
    measured over one 19-session run, sessions ran `grep -n "<count>" docs/implementation-plan.md`
    twelve times, in the tail, to find where the number they had just moved was written down. The
    substitution is mechanical -- the tree knows the new number and the field holds the old one --
    so `--template` hands it over ready to apply instead.

    A `new` with an empty `old` is a count that could not be quoted uniquely: still reported, but
    the session has to write that fragment itself.
    """
    c = counts()
    out: list[tuple[str, str, str]] = []
    for name, _a, _b, body in plan_fields():
        for label, value in (("conformance", c["conformance"]),
                             ("differential", c["differential"])):
            for m in re.finditer(rf"{label}\D{{0,12}}(\d{{2,4}})", body, flags=re.I):
                if int(m.group(1)) == value:
                    continue
                span = unique_span(body, m.start(), m.end())
                if span is None:
                    out.append((name, "", f"{label} {m.group(1)} -> {value}"))
                    continue
                lo, hi = span
                old = body[lo:hi]
                at = m.start(1) - lo
                out.append((name, old, old[:at] + str(value) + old[at + len(m.group(1)):]))
    return out


def playbook_headings() -> list[str]:
    """The `## ` headings a `## playbook:` section may name.

    Thirteen `grep -n "^## "` calls over the playbook in one 19-session run, every one of them in
    the tail, asking a question that has the same answer every time.
    """
    return [head for _dir, head in playbookmod.SECTIONS]


def check() -> int:
    c = counts()
    say("== COUNTS  (what the plan's prose should agree with)")
    say(f"  conformance cases   {c['conformance']}")
    say(f"  differential cases  {c['differential']}")
    say(f"  ADRs on disk        {c['adrs']}, highest {c['highest_adr']:04d}, "
        f"next free {c['highest_adr'] + 1:04d}")

    say()
    aim = planmod.field_aim()
    ceiling = planmod.field_ceiling()
    say(f"== PLAN  (fields, any that name a stale count, and what each costs; aim ~{aim} B, "
        f"ceiling {ceiling} B)")
    edits = stale_edits()
    stale = len(edits)
    total = 0
    for name, _a, _b, body in plan_fields():
        flag = ""
        mine = [e for e in edits if e[0] == name]
        if mine:
            flag = f"   <- {len(mine)} stale count(s); `--template` hands them back ready to apply"
        n = len(body.encode("utf-8"))
        total += n
        if n > ceiling:
            flag += f"   OVER the ceiling: an edit that grows it is refused -- replace, do not add"
        elif not flag and n > aim * 1.5:
            flag = f"   {n / aim:.0f}x the aim, {ceiling - n} B of headroom"
        say(f"  {name:<18} {n:>5} B{flag}")
    say(f"  {'':<18} {total:>5} B  shipped into every session. `--wrap` refuses an edit that")
    say("                          leaves a field both over the ceiling and bigger than it was;")
    say("                          a shrink is always taken. `plan.py --check` is the same number.")

    say()
    say("== HANDOFF")
    if not HANDOFF.exists():
        say("  MISSING")
    else:
        body = HANDOFF.read_text(encoding="utf-8")
        n = len(body.split("\n"))
        problems = validate_handoff(body)
        say(f"  {n} lines" + (f" (target ~{HANDOFF_TARGET_LINES})" if n > 85 else ""))
        for p in problems:
            say(f"  - {p}")
        if not problems:
            say("  shape OK: State / Next group / Backlog, every open item with a repo-rooted file:NN anchor")

    say()
    say("== LINKS  (python tools/check-links.py -- CI's `docs` job, which nv verify does not run)")
    broke, found = link_findings()
    base = session_base()
    where = "HEAD" if base == "HEAD" else f"{base[:9]}, where this session opened"
    for ln in broke:
        say(f"  YOURS  {ln}")
    for ln in found[:8]:
        say(f"  inherited  {ln}")
    if len(found) > 8:
        say(f"  ... and {len(found) - 8} more that were already dead at {where}")
    if not broke and not found:
        say("  every link resolves, with matching case and form")
    elif not broke:
        say(f"  none of the {len(found)} is this session's -- all were dead at {where} -- "
            f"so `--wrap` will not refuse over them")
    else:
        say("  `--wrap` refuses while a YOURS line stands: fix the link, not the citation's line")

    say()
    say("== RULEBOOK  (python tools/rules.py -- the same job, and only when you edited docs/rules/)")
    rule_problems = rulebook_findings()
    if not rule_problems:
        touched = git("status", "--porcelain", "--", "docs/rules", check=False)
        say("  clean -- this wrap is not gated on it"
            if touched.returncode != 0 or not touched.stdout.strip()
            else "  every rule loads, every citation resolves, and the rendered pages are current")
    for p in rule_problems:
        say(f"  YOURS  {p}")

    say()
    say("== RECORDS  (python tools/records.py -- CI's `docs` job, and only when you edited them)")
    record_problems = record_findings()
    if not record_problems:
        touched = git("status", "--porcelain", "--", "docs/decisions", check=False)
        say("  clean -- this wrap is not gated on it"
            if touched.returncode != 0 or not touched.stdout.strip()
            else "  every record's field set, heading order and derived counters are right")
    for p in record_problems:
        say(f"  YOURS  {p}")

    say()
    say("== MIGRATION  (python tools/check-migration.py -- only when you edited docs/spec/)")
    migration_problems = migration_findings()
    if not migration_problems:
        touched = git("status", "--porcelain", "--", "docs/spec", check=False)
        say("  clean -- this wrap is not gated on it"
            if touched.returncode != 0 or not touched.stdout.strip()
            else "  every row of the migration table is well formed")
    for p in migration_problems:
        say(f"  YOURS  {p}")

    say()
    say("== TREE")
    st = _checked(["git", "status", "--short"]).stdout.strip()
    if not st:
        say("  clean -- nothing to commit")
    else:
        for ln in st.split("\n")[:20]:
            say(f"  {ln}")
        extra = len(st.split("\n")) - 20
        if extra > 0:
            say(f"  ... and {extra} more")
    say()
    say("Write one wrap file and apply it with `python tools/session.py --wrap <file>`;")
    say("its format is this tool's --help.")
    return 1 if stale or broke else 0


# ---------------------------------------------------------------------------- main


def template() -> int:
    """A wrap file skeleton, with this tree's counts already in it, ready to fill in and apply.

    Sessions were spending two tail calls -- `session.py --help` and `plan.py --help` -- recovering
    this format at the point where context is most expensive. Inlining the format into orient.py's
    pack instead would cost more than it saves: the help is 4 KB, and a pack is re-billed on every
    one of a session's ~98 calls, against two calls paid once. So it lives here, reachable from a
    call the tail already makes.

    Every placeholder is the right *shape*, so `--wrap --dry-run` on the unedited skeleton reports
    only the one thing a template cannot know -- that `## commit:`'s paths do not exist yet. Anything
    else it reports is something the fill-in got wrong.
    """
    c = counts()
    edits = [] if SIDE else stale_edits()
    if SIDE:
        pass  # a side run writes no plan section, and `validate` refuses one
    elif edits:
        # Every count the tree has moved past, already written as an applicable edit. This is the
        # one section a counted goal's session always needs and always used to derive by hand.
        # Nothing but the pairs goes in these sections: `parse_edits` refuses text in front of a
        # `--- old` and folds text after a `--- new` into the fragment, so a note here would
        # corrupt the very edit it was explaining. The guidance lives in --help and --check.
        for field in dict.fromkeys(name for name, _o, _n in edits):
            say(f"## plan-edit: {field}")
            for _f, old, new in [e for e in edits if e[0] == field]:
                say("--- old")
                say(old or f"QUOTE THE SENTENCE HOLDING {new} -- widening past {QUOTE_MAX} chars")
                say("--- new")
                say(new)
            say("")
    else:
        # Nothing is stale, so the skeleton shows the SHAPE of a quote instead. The `--- old` is
        # a real run of words out of the live field, so the unedited skeleton still validates.
        field = next((t for n, _a, _b, t in plan_fields() if n == "Open now"), "")
        sample = " ".join(field.split()[:9]) or "the run of words that is now wrong"
        ceiling = planmod.field_ceiling()
        say("## plan-edit: Open now")
        say("--- old")
        say(sample)
        say("--- new")
        say(f"{sample}   <- REPLACE both fragments. Repeat the pair per place the field moved.")
        say(f"The tree's counts are {', '.join(f'{k} {v}' for k, v in c.items())} and no plan field")
        say("names a stale one. `## plan: <Field>` replaces a field WHOLE instead, for a real")
        say("rewrite -- a big replacement mostly already on disk is refused as a retype.")
        say(f"Open now is {nbytes(field)} B of its {ceiling} B ceiling: an edit that")
        say("leaves it both bigger than now AND over that is refused, so replace a sentence")
        say("rather than adding one -- an empty `--- new` drops the one that went stale.")
    say("")
    say("## playbook: Tooling")
    say("- **DELETE THIS SECTION unless a trap cost you time.** A bullet is appended under the")
    say("  heading, never rewritten, so only add one that is not already there -- three sentences,")
    say("  trap / why / what to do (conventions.md § A playbook bullet), not the session's story,")
    say("  which git log holds. The headings are")
    for heading in playbook_headings():
        say(f"  {heading}")
    say("")
    say("## handoff")
    say("## State")
    say("REPLACE. Where the work stands now -- not the path taken to get here.")
    say("")
    say("## Next group")
    say("- [ ] **The claim** -- anchored, as `crates/nvs-stdlib/src/arr.rs:2084`, so the next")
    say("      session does not re-derive what this one already had open.")
    say("")
    say("## Backlog")
    say("- What this session did not take.")
    say("")
    say("## commit: path/one.rs path/two.rs")
    say("test(stdlib): what is now true, lower case, no trailing period")
    say("")
    say("The body. No trailers of any kind -- they are stripped and counted.")
    say("")
    # The docs this wrap writes, committed by this wrap. It is pre-filled because leaving it to
    # be remembered did not work: 9 of 19 sessions in one run ended with a hand-rolled `git add`
    # of exactly these three paths, after the wrap had already written all three.
    docs = [playbookmod.PLAYBOOK_DIR, HANDOFF] if SIDE else [PLAN, playbookmod.PLAYBOOK_DIR, HANDOFF]
    say(f"## commit: {' '.join(rel_path(p) for p in docs)}")
    say("docs(agent): what the handoff now says" if SIDE
        else "docs(agent): what the plan and the handoff now say")
    say("")
    say("Drop a path this wrap does not write. Anything it does write that no `## commit:`")
    say("names joins the last one regardless, so the docs cannot be left dirty.")
    say("")
    say("## status")
    say("CONTINUE one line saying what landed")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--wrap", metavar="FILE", help="apply a wrap file: the whole session tail")
    ap.add_argument("--dry-run", action="store_true", help="validate and report, write nothing")
    ap.add_argument("--check", action="store_true", help="what step 4-5 still owes")
    ap.add_argument("--counts", action="store_true", help="conformance / differential / ADR counts")
    ap.add_argument("--template", action="store_true",
                    help="print a wrap file skeleton to fill in, with this tree's counts already in it")
    ap.add_argument("--scrub", action="store_true",
                    help="read a commit message on stdin, write it trailer-free on stdout")
    opts = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    try:
        if opts.scrub:
            # A stdin/stdout filter so the same rule can clean history, not just new commits:
            #   git filter-branch -f --msg-filter "python tools/session.py --scrub" <range>
            try:
                sys.stdin.reconfigure(encoding="utf-8", errors="replace")
            except AttributeError:
                pass
            text, _n = strip_trailers(sys.stdin.read())
            sys.stdout.write(text)
            return 0
        if opts.counts:
            for k, v in counts().items():
                say(f"{k} {v}")
            return 0
        if opts.template:
            return template()
        if opts.check:
            return check()
        if opts.wrap:
            return wrap(Path(opts.wrap), opts.dry_run)
    except RuntimeError as exc:
        say(f"session.py: {exc}")
        return 2
    ap.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
