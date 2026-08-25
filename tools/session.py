#!/usr/bin/env python3
"""AGENTS.md § *Session workflow* steps 4 and 5, as one call.

Measured over a full loop run, the tail of a session -- everything from the first `verify.py` to
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

    ## plan: Open now
    What the plan's `Open now` field should now say. One paragraph; it is re-wrapped.

    ## plan: On disk
    Same, for another field. Name as many `## plan:` sections as you changed, no more.

    ## playbook: Tooling
    - **A new trap, as a bullet.** Appended under that heading, never rewriting what is there.

    ## handoff
    ## State
    ...the whole handoff body, verbatim, replacing the file...

    ## commit: crates/mwl-ir/src/lower/expr.rs crates/mwl-ir/src/ir.rs
    feat(ir): the subject line

    The body, if there is one.

    ## commit: docs/implementation-plan.md docs/agent/handoff.md
    docs(agent): the second slice's own commit

    ## status
    CONTINUE one line saying what landed

Applied in this order, and **nothing is applied until every section validates**: plan fields,
playbook, handoff, commits in the order written, then `status.txt`. A `## commit:` whose paths
match nothing staged is an error before the first field is touched, not a half-finished tail.

## The rest

    python tools/session.py --check          # what step 4-5 still owes, from the tree
    python tools/session.py --counts         # conformance / differential / ADR counts, ready to paste
    python tools/session.py --wrap F --dry-run    # say what it would do, touch nothing

A commit message documents the work and nothing else. Any `Co-Authored-By`, `Signed-off-by`,
`Generated-with` or similar attribution line in a `## commit:` body is **stripped** before the
commit is made, and the count is reported. conventions.md § *A commit message* is the rule;
`tools/git-hooks/commit-msg` catches the same thing arriving by any other route.

`--check` is the one to run *before* writing the wrap file: it says which plan fields have gone
stale against the tree, whether the handoff still matches its contract, and what is uncommitted.

This tool judges no content. It refuses malformed input and it refuses to invent a plan field --
everything else it writes is what you handed it.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
AGENT = DOCS / "agent"
PLAN = DOCS / "implementation-plan.md"
PLAYBOOK = AGENT / "playbook.md"
HANDOFF = AGENT / "handoff.md"
RUNDIR = ROOT / ".loop"
STATUS = RUNDIR / "status.txt"
TMP = ROOT / ".agent-tmp"

HANDOFF_REQUIRED = ["## State", "## Next group", "## Backlog"]
HANDOFF_TARGET_LINES = 60
STATUS_WORDS = ("CONTINUE", "DONE", "BLOCKED")

sys.path.insert(0, str(Path(__file__).resolve().parent))
import plan as planmod  # noqa: E402  -- the status block's one home; never reimplemented here


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
    __slots__ = ("kind", "arg", "body", "line")

    def __init__(self, kind: str, arg: str, body: str, line: int):
        self.kind, self.arg, self.body, self.line = kind, arg, body, line


def parse_wrap(text: str) -> tuple[list[Section], list[str]]:
    """`## kind: arg` headings and their bodies, in file order.

    The handoff body is markdown containing its own `## ` headings, so the parser has to stop
    treating `## ` as a directive once it is inside `## handoff` -- otherwise the handoff's own
    `## State` would read as an unknown instruction. It resumes at the next *known* directive."""
    known = ("plan", "playbook", "handoff", "commit", "status")
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
        m = re.match(r"^##\s+([A-Za-z]+)\s*:?\s*(.*)$", raw)
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
        if s.kind in ("plan", "playbook", "commit") and not s.arg:
            errors.append(f"line {s.line}: `## {s.kind}:` needs an argument")
        if s.kind in ("handoff", "status") and s.arg:
            errors.append(f"line {s.line}: `## {s.kind}` takes no argument, got {s.arg!r}")
        if not s.body.strip():
            errors.append(f"line {s.line}: `## {s.kind}` has an empty body")
    return sections, errors


# ------------------------------------------------------------------------ validation


def plan_fields() -> list[tuple[str, int, int, str]]:
    _text, lines = planmod.load()
    return planmod.find_fields(lines)


def validate(sections: list[Section]) -> list[str]:
    """Everything that could refuse, refusing here -- before a single byte is written."""
    errors: list[str] = []
    names = {n.lower(): n for n, _a, _b, _t in plan_fields()}

    for s in sections:
        if s.kind == "plan":
            if s.arg.lower() not in names:
                errors.append(
                    f"`## plan: {s.arg}` -- no such field, and this tool does not add one. "
                    f"The block has: {', '.join(names.values())}")
        elif s.kind == "playbook":
            text = PLAYBOOK.read_text(encoding="utf-8")
            if not heading_index(text, s.arg):
                heads = re.findall(r"^## (.+)$", text, flags=re.M)
                errors.append(f"`## playbook: {s.arg}` -- no such heading. "
                              f"playbook.md has: {', '.join(heads)}")
            if not s.body.lstrip().startswith("-"):
                errors.append(f"`## playbook: {s.arg}` -- a playbook entry is a `- ` bullet")
        elif s.kind == "status":
            first = s.body.strip().split("\n")[0]
            if not first.startswith(STATUS_WORDS):
                errors.append(f"`## status` -- must start with one of {'/'.join(STATUS_WORDS)}, "
                              f"got {first[:40]!r}")
            if len(s.body.strip().split("\n")) > 1:
                errors.append("`## status` -- one line only")
        elif s.kind == "commit":
            errors.extend(validate_commit(s))
        elif s.kind == "handoff":
            errors.extend(validate_handoff(s.body))
    return errors


def validate_commit(s: Section) -> list[str]:
    errors = []
    subject = s.body.strip().split("\n")[0]
    if not re.match(r"^(feat|fix|docs|test|perf|refactor|chore|build|ci)"
                    r"(\([a-z0-9-]+\))?: .+", subject):
        errors.append(f"`## commit:` line {s.line} -- subject is not "
                      f"`type(scope): subject`: {subject[:60]!r}")
    if len(subject) > 120:
        errors.append(f"`## commit:` line {s.line} -- subject is {len(subject)} chars")
    for path in s.arg.split():
        target = ROOT / path
        if not target.exists() and "*" not in path:
            errors.append(f"`## commit:` line {s.line} -- no such path: {path}")
    return errors


def validate_handoff(body: str) -> list[str]:
    errors = []
    for required in HANDOFF_REQUIRED:
        if not re.search(rf"^{re.escape(required)}\b", body, flags=re.M):
            errors.append(f"`## handoff` -- missing the required `{required}` heading")
    if not re.search(r"\.rs:\d+|\.md:\d+|\.py:\d+", body):
        errors.append("`## handoff` -- `## Next group` carries no `file:NN` anchor. "
                      "They are not optional; resolve them with `python tools/peek.py --locate`.")
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


# ---------------------------------------------------------------------------- apply


def apply_plan(s: Section, dry: bool) -> str:
    text, lines = planmod.load()
    fields = planmod.find_fields(lines)
    target = next(f for f in fields if f[0].lower() == s.arg.lower())
    name, start, end, old = target
    new = " ".join(s.body.split())
    if dry:
        return f"plan: {name}  {len(old)} -> {len(new)} bytes"
    rewritten = lines[:start] + ["> " + ln for ln in planmod.render(name, new)] + lines[end:]
    PLAN.write_text("\n".join(rewritten), encoding="utf-8", newline="")
    return f"plan: {name}  {len(old)} -> {len(new)} bytes"


def apply_playbook(s: Section, dry: bool) -> str:
    text = PLAYBOOK.read_text(encoding="utf-8")
    span = heading_index(text, s.arg)
    assert span is not None  # validate() proved it
    lines = text.split("\n")
    _start, end = span
    # Append at the end of the section's own bullets, before whatever heading follows, keeping
    # exactly one blank line ahead of that heading. This file is append-mostly by decision
    # (AGENTS.md § *Keep each slice small*); nothing here rewrites a bullet that is already there.
    at = end
    while at > 0 and not lines[at - 1].strip():
        at -= 1
    bullet = s.body.rstrip().split("\n")
    if dry:
        return f"playbook: + {len(bullet)} line(s) under {s.arg!r}"
    lines[at:at] = bullet
    PLAYBOOK.write_text("\n".join(lines), encoding="utf-8", newline="")
    return f"playbook: + {len(bullet)} line(s) under {s.arg!r}"


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
    if dry:
        return f"commit: {subject[:70]}  ({len(paths)} path(s))"
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
    return f"commit: {head}  ({len(staged)} file(s)){note}"


def apply_status(s: Section, dry: bool) -> str:
    line = s.body.strip()
    if not RUNDIR.is_dir():
        return "status: skipped -- no .loop directory (not a loop session)"
    if not dry:
        STATUS.write_text(line, encoding="utf-8", newline="\n")
    return f"status: {line[:80]}"


APPLY = {"plan": apply_plan, "playbook": apply_playbook, "handoff": apply_handoff,
         "commit": apply_commit, "status": apply_status}
ORDER = ["plan", "playbook", "handoff", "commit", "status"]


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
    say(f"session.py: {'would apply' if dry else 'applied'} {len(ordered)} section(s)")
    for s in ordered:
        say(f"  {APPLY[s.kind](s, dry)}")
    if not dry:
        left = _checked(["git", "status", "--short"]).stdout.strip()
        if left:
            say(f"  uncommitted after the wrap ({len(left.split(chr(10)))} path(s)):")
            for ln in left.split("\n")[:12]:
                say(f"    {ln}")
    return 0


# --------------------------------------------------------------------------- check


def counts() -> dict[str, int]:
    def mwlt(sub: str) -> int:
        d = ROOT / "tests" / sub
        return len(list(d.rglob("*.mwlt"))) if d.is_dir() else 0
    adrs = sorted((DOCS / "adr").glob("[0-9][0-9][0-9][0-9]-*.md"))
    return {
        "conformance": mwlt("conformance"),
        "differential": mwlt("differential"),
        "adrs": len(adrs),
        "highest_adr": int(adrs[-1].name[:4]) if adrs else 0,
    }


def check() -> int:
    c = counts()
    say("== COUNTS  (what the plan's prose should agree with)")
    say(f"  conformance cases   {c['conformance']}")
    say(f"  differential cases  {c['differential']}")
    say(f"  ADRs on disk        {c['adrs']}, highest {c['highest_adr']:04d}, "
        f"next free {c['highest_adr'] + 1:04d}")

    say()
    say("== PLAN  (fields, and any that name a stale count)")
    stale = 0
    for name, _a, _b, body in plan_fields():
        flag = ""
        for label, value in (("conformance", c["conformance"]), ("differential", c["differential"])):
            for found in re.findall(rf"{label}\D{{0,12}}(\d{{2,4}})", body, flags=re.I):
                if int(found) != value:
                    flag = f"   <- says {label} {found}, tree has {value}"
                    stale += 1
        say(f"  {name:<18} {len(body):>5} B{flag}")

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
            say("  shape OK: State / Next group / Backlog, with file:NN anchors")

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
    return 1 if stale else 0


# ---------------------------------------------------------------------------- main


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--wrap", metavar="FILE", help="apply a wrap file: the whole session tail")
    ap.add_argument("--dry-run", action="store_true", help="validate and report, write nothing")
    ap.add_argument("--check", action="store_true", help="what step 4-5 still owes")
    ap.add_argument("--counts", action="store_true", help="conformance / differential / ADR counts")
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
