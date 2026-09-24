#!/usr/bin/env python3
"""Read many places in many files in ONE tool call.

Measured over a full loop run: **0 of 3,647** tool-call messages carried more than one call,
against an AGENTS.md rule that asks for exactly that. Thirty-nine sessions read that rule and
none applied it, including runs of 52 and 57 consecutive `grep`/`sed` calls. A later 33-session run
measured the same 1.00 calls per message. That experiment is finished: a rule the model must
remember and choose loses to a tool it can just call. `bun nv splice` is the same move for the
write side, where runs of consecutive `Edit` calls were costing 10.3 turns a session.

So this is batching as a *tool*. One call, N targets, N answers -- and the batching happens
whether or not anyone thought about it.

    python tools/peek.py crates/nvs-ir/src/lower/expr.rs:3065-3120 \\
                         crates/nvs-types/src/expr/members.rs:@public_property_names \\
                         docs/decisions/0036.md:"### 4" \\
                         "crates/nvs-runtime/src/*.rs:/slot_get/"

Target forms, all of them `path` followed by `:` and a locator:

    path                  the whole file (refused over --max-lines, which says so and stops)
    path:120-160          those lines
    path:120+30           30 lines starting at 120
    path:@name            the line that *defines* `name`, plus --window lines of context
    path:re:regex         every matching line and nothing around it, `grep -n` style -- a match
                          on a heading or a `//!` line almost always wants context
    path:re:regex:3       every matching line with 3 lines of context either side; `--context 3`
                          says that for every target in the call, and a suffix wins over it
    path:/regex/          the same, in the familiar spelling -- but Git Bash on Windows
                          rewrites a leading `/` into a Win32 path before this tool sees it,
                          so prefer `re:` there
    path:"## Heading"     a markdown heading and its body, to the next same-or-higher heading
    rule:topic/slug       that rule's fragment -- the citation token itself, as a target
    rule:topic            the whole generated chapter (usually too big; name the rule instead)

`path` may be a glob (`crates/**/*.rs`), in which case the locator runs against every match --
which is how you sweep a regex across a crate without a second call.

A `rule:` target takes no locator, because a fragment is a page or two and the whole point is that
the token you are already looking at *is* the target -- paste it, backticks and all, beside the
code targets you were going to read anyway. `docs/rules/<topic>.md:"## …"` is still there for a
slice of a chapter. (The two forms above are metasyntax, not citations, so this file carries
`rules-py:examples` -- `tools/rules.py`'s marker for a document that teaches the spelling.)

    python tools/peek.py --locate nvs_object_slot_get SlotSet ClassDesc
    python tools/peek.py --outline crates/nvs-ir/src/lower/mod.rs

`--locate` is the other half: symbols in, `file:line  <the defining line>` out, and no bodies at
all. It is what a handoff's `## Next group` file set is made of, and what the tail of a session
otherwise spends five `grep`s rediscovering.

`--outline` is for the file you do not know yet: one line per `fn`/`struct`/`enum`/`trait`/`impl`
seam, with the line it starts on and how long it runs. **56% of a session's read calls re-fetch a
file it has already opened** -- 24.6 calls a session over only 20.3 distinct files -- and the two
hottest files here are 5,044 and 5,720 lines, so "read it whole" is not available. Landing the
first fetch correctly is what is available, and an outline is the map that does it: 2.8 KB against
`lower/mod.rs`'s 276 KB. Top-level seams only unless `--deep`. When a session has fetched the same
file three times, the footer says so and names this flag.

A second count rides the same ledger. **51% of `peek.py` calls carry a single target**, measured
over the 32-session run of 2026-08-27, against 41 read calls a session -- so the tool that exists
to batch is being called the way the thing it replaced was. A session's clock is 90% round trips,
which makes four one-target calls in a row four of them for bytes one call would have carried, and
the footer says so. Batch what you have already decided to read; never what the previous result is
about to tell you.

Nothing here judges or truncates silently. Every target that produced nothing says so on its own
line, and a target that would blow the budget prints its size and refuses rather than quietly
handing back half a file. The footer says what the call cost, because the point of the tool is
that the cost is visible in one place instead of spread over fifty turns.
"""

from __future__ import annotations

import argparse
import glob as globmod
import json
import os
import re
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Calibrated in tools/loop-stats.py against real transcripts: source text in this repository
# tokenizes at about 2.5 bytes per token. Used only to print a cost, never to refuse one.
BYTES_PER_TOKEN = 2.5

DEFAULT_WINDOW = 12
DEFAULT_MAX_LINES = 400  # AGENTS.md rule 3's "whole file under ~400 lines"

# What counts as *defining* a name, across the languages this repository actually holds. A hit
# here beats a plain mention, which is the whole difference between `--locate` and a `grep`.
DEFINITION = [
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+|unsafe\s+|extern\s+\"[^\"]*\"\s+)*fn\s+{name}\b",
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait|union|type|mod|macro_rules!)\s+{name}\b",
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:static|const)\s+{name}\b",
    r"^\s*impl\b.*\b{name}\b",
    r"^\s*{name}\s*[{{(,]",                   # an enum variant, which has no other declarer
    r"^\s*(?:def|class)\s+{name}\b",          # python
    r"^\s*{name}\s*=",                        # a top-level binding, python or toml
    r"^#+\s*.*\b{name}\b",                    # a markdown heading naming it
]

SKIP_DIRS = {".git", "target", "node_modules", "__pycache__", ".agent-tmp", ".loop"}
TEXT_SUFFIXES = {".rs", ".py", ".md", ".toml", ".nvs", ".nvst", ".txt", ".json", ".yml",
                 ".yaml", ".sh", ".ps1", ".snap", ".php", ".lock", ".cfg", ".ini"}

# What `--outline` prints one line for: the seams of a file, and nothing inside them. A `fn`, a
# `struct`, an `impl` -- the things a `:@name` target can then land on exactly.
OUTLINE = re.compile(
    r"^(?P<indent>\s*)(?P<sig>"
    r"(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+|unsafe\s+|extern\s+\"[^\"]*\"\s+)*fn\s+\w+"
    r"|(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait|union|mod)\s+\w+"
    r"|impl(?:<[^>]*>)?\s+[^{;]+"
    r")"
)

#: Python's seams, kept separate because `class Adder` is also Novis, and this repository's Rust is
#: full of Novis fixtures in string literals -- matching them turned a `lower/mod.rs` outline into
#: a list of test classes.
OUTLINE_PY = re.compile(r"^(?P<indent>\s*)(?P<sig>(?:async\s+)?(?:def|class)\s+\w+)")

#: Where this session's fetches are remembered, so a repeat can be reported. One file per
#: process tree is not possible -- `peek.py` is a fresh process every call -- so it is one file
#: per day under `.agent-tmp/`, which is gitignored and swept with the rest of it.
LEDGER = ROOT / ".agent-tmp" / "peek-ledger.json"

#: Fetches of one file, in one session, past which the footer says so. Three is where reading
#: the seams first (`--outline`) starts to beat guessing at another region.
REFETCH_NOTE_AT = 3

#: Single-target calls in a row past which the footer says so. Four, because a run of three is a
#: plausible chain of genuinely undecided reads -- each target chosen by what the last one
#: returned, which is the case this tool must not nag about. By four, a session is walking a file
#: set it already knew when it started.
SOLO_NOTE_AT = 4

#: Idle time that ends a session when there is no loop driver to ask. `.agent-tmp` is never
#: swept, so without a boundary the tally would be cumulative and the advice would fire on every
#: call of every session forever.
LEDGER_IDLE_SECONDS = 2 * 60 * 60


def session_key() -> str:
    """What makes this session distinct from the last one, for the re-fetch ledger.

    Under the loop driver, consecutive sessions are 0.4 minutes apart -- far too close for an
    idle timer to separate -- but each one has its own transcript, so the newest log file names
    it. Interactively there is no such marker and the idle timer is the whole answer."""
    logs = ROOT / ".loop" / "logs"
    if logs.is_dir():
        newest = max(logs.glob("*.log"), key=lambda p: p.stat().st_mtime, default=None)
        if newest is not None:
            return newest.name
    return "interactive"


# ----------------------------------------------------------------------------- plumbing


def out(line: str = "") -> None:
    print(line)


def read_lines(path: Path) -> list[str] | None:
    try:
        return path.read_text(encoding="utf-8", errors="replace").split("\n")
    except OSError:
        return None


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def walk_repo():
    """Every text file under the repo, minus the directories no question is ever about."""
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in filenames:
            p = Path(dirpath) / name
            if p.suffix in TEXT_SUFFIXES:
                yield p


#: `rule:<topic>/<slug>` -- the citation token, spelled exactly as the ~18,500 of them in the tree
#: are. It is the most-copied identifier in this repository and, until this rewrite, the one thing
#: `peek.py` could not be handed: a session reading `rule:types/conversion` in a doc comment had to
#: know that a rule's prose is a fragment at `docs/rules/<topic>/<slug>.md` and that the topic
#: chapter beside it is generated. Optional backticks, because that is how a doc comment writes it
#: and pasting one back with them attached is the obvious mistake to absorb rather than report.
RULE_TARGET = re.compile(r"^`?rule:([a-z0-9][a-z0-9-]*)(?:/([a-z0-9][a-z0-9-]*))?`?$")


def rule_target(spec: str) -> str | None:
    """`rule:types/conversion` -> the fragment path, `rule:types` -> the chapter. Else `None`.

    A pure string rewrite, with no rulebook load behind it: the id *is* the path, which is
    `tools/rules.py`'s own § *A rule id is a path*. A mistyped id therefore lands on this tool's
    ordinary NO SUCH FILE, and the caller below adds where the real list is.
    """
    m = RULE_TARGET.match(spec.strip())
    if not m:
        return None
    topic, slug = m.group(1), m.group(2)
    return f"docs/rules/{topic}/{slug}.md" if slug else f"docs/rules/{topic}.md"


def split_target(spec: str) -> tuple[str, str | None]:
    """`path:locator` -> (path, locator), tolerating a Windows drive letter and a bare path.

    The split is on the FIRST colon past a drive letter, not the last: a locator may itself
    contain colons (`re:fn foo`) while a path essentially never does."""
    start = 2 if len(spec) > 1 and spec[1] == ":" and spec[0].isalpha() else 0
    i = spec.find(":", start)
    if i < 0 or i == len(spec) - 1:
        return spec, None
    return spec[:i], spec[i + 1 :]


def expand(pattern: str) -> list[Path]:
    """A path or a glob -> the files it names, repo-relative or absolute, sorted."""
    p = Path(pattern)
    if p.exists() and p.is_file():
        return [p]
    candidates = globmod.glob(pattern, recursive=True)
    if not candidates:
        candidates = globmod.glob((ROOT / pattern).as_posix(), recursive=True)
    return sorted(Path(c) for c in candidates if Path(c).is_file())


# ----------------------------------------------------------------------------- locators


def locate_definition(lines: list[str], name: str) -> int | None:
    """The line index that defines `name`, or the first mention if nothing defines it."""
    for template in DEFINITION:
        rx = re.compile(template.format(name=re.escape(name)))
        for i, line in enumerate(lines):
            if rx.search(line):
                return i
    for i, line in enumerate(lines):
        if re.search(rf"\b{re.escape(name)}\b", line):
            return i
    return None


def heading_span(lines: list[str], wanted: str) -> tuple[int, int] | None:
    """A markdown heading and its body, to the next heading at the same or a higher level.

    Matches on the heading's *words*, so `## 4`, `4`, and `4. What a write checks` all find the
    same section -- the same normalisation orient.py uses, for the same reason, including why the
    letter of a `3a` stays attached to its number."""
    def norm(t: str) -> str:
        t = re.sub(r"[`*_#]", "", t).strip().lower()
        t = re.sub(r"^(\d+[a-z]?)\s*[.)]?\s*", r"\1 ", t)
        return re.sub(r"\s+", " ", t).strip()

    key = norm(wanted)
    heads = [(i, len(m.group(1)), m.group(2))
             for i, line in enumerate(lines)
             if (m := re.match(r"^(#{1,6})\s+(.*)$", line))]
    for n, (idx, level, title) in enumerate(heads):
        t = norm(title)
        if t == key or t.startswith(key + " ") or t.startswith(key + "."):
            end = len(lines)
            for later_idx, later_level, _ in heads[n + 1 :]:
                if later_level <= level:
                    end = later_idx
                    break
            return idx, end
    return None


def parse_range(locator: str, total: int) -> tuple[int, int] | None:
    m = re.fullmatch(r"(\d+)-(\d+)", locator)
    if m:
        a, b = int(m.group(1)), int(m.group(2))
        return max(0, a - 1), min(total, b)
    m = re.fullmatch(r"(\d+)\+(\d+)", locator)
    if m:
        a, n = int(m.group(1)), int(m.group(2))
        return max(0, a - 1), min(total, a - 1 + n)
    m = re.fullmatch(r"(\d+)", locator)
    if m:
        a = int(m.group(1))
        return max(0, a - 1), min(total, a)
    return None


# ------------------------------------------------------------------------------ emitting


def emit_span(path: Path, lines: list[str], start: int, end: int, note: str) -> int:
    """One numbered window, with a header naming what asked for it. Returns bytes printed."""
    header = f"===== {rel(path)}:{start + 1}-{end}  {note}"
    out(header)
    width = len(str(end))
    body = []
    for i in range(start, min(end, len(lines))):
        body.append(f"{i + 1:>{width}}  {lines[i]}")
    for b in body:
        out(b)
    out()
    return len(header) + sum(len(b) + 1 for b in body)


def emit_matches(path: Path, lines: list[str], rx, context: int) -> int:
    """`grep -n` with optional context, collapsed so overlapping windows print once."""
    hits = [i for i, line in enumerate(lines) if rx.search(line)]
    if not hits:
        return 0
    spans: list[list[int]] = []
    for i in hits:
        a, b = max(0, i - context), min(len(lines), i + context + 1)
        if spans and a <= spans[-1][1]:
            spans[-1][1] = max(spans[-1][1], b)
        else:
            spans.append([a, b])
    printed = 0
    header = f"===== {rel(path)}  /{rx.pattern}/  {len(hits)} hit(s)"
    out(header)
    printed += len(header)
    width = len(str(len(lines)))
    for n, (a, b) in enumerate(spans):
        if n:
            out("  --")
            printed += 4
        for i in range(a, b):
            line = f"{i + 1:>{width}}  {lines[i]}"
            out(line)
            printed += len(line) + 1
    out()
    return printed


def peek_one(spec: str, window: int, max_lines: int, context: int) -> tuple[int, int]:
    """One target -> (bytes printed, targets that produced nothing)."""
    as_rule = rule_target(spec)
    pattern, locator = split_target(as_rule if as_rule else spec)
    files = expand(pattern)
    if not files:
        where = f"{pattern}  -- NO SUCH FILE"
        if as_rule:
            where = (f"{spec}  -- NO SUCH RULE (looked in {pattern}). "
                     f"`python tools/rules.py --list` is every rule id.")
        out(f"===== {where}")
        out()
        return 0, 1

    # Over a glob, a file with no hit is the normal case, not a miss: a sweep of eight modules
    # for one symbol is *supposed* to be quiet in seven of them. So a sweep counts as empty only
    # when nothing anywhere matched.
    sweep = len(files) > 1
    printed, empty, sweep_hits = 0, 0, 0
    for path in files:
        lines = read_lines(path)
        if lines is None:
            out(f"===== {rel(path)}  -- UNREADABLE")
            out()
            empty += 1
            continue
        # A trailing newline gives a phantom last element; drop it so counts are honest.
        if lines and lines[-1] == "":
            lines.pop()

        if locator is None:
            if len(lines) > max_lines:
                out(f"===== {rel(path)}  -- {len(lines)} lines, over --max-lines "
                    f"{max_lines}. Name a region: `{rel(path)}:1-{max_lines}`, "
                    f"`{rel(path)}:@symbol`, or `{rel(path)}:/regex/`.")
                out()
                empty += 1
                continue
            printed += emit_span(path, lines, 0, len(lines), "whole file")
            continue

        if locator.startswith("@"):
            name = locator[1:]
            idx = locate_definition(lines, name)
            if idx is None:
                if not sweep:
                    out(f"===== {rel(path)}  -- no definition or mention of `{name}`")
                    out()
                    empty += 1
                continue
            sweep_hits += 1
            a, b = max(0, idx - 2), min(len(lines), idx + window)
            printed += emit_span(path, lines, a, b, f"@{name}")
            continue

        # `/re/` is the familiar spelling; `re:` is the one that survives Windows. Git Bash
        # rewrites any argument that *starts* with a slash into a Win32 path before this process
        # ever sees it, so `path:/fn foo/` arrives as `path;C:/Program Files/Git/fn foo/`. Both
        # forms mean the same thing and `re:` is the one to reach for here.
        m = re.fullmatch(r"/(.*)/(\d*)", locator, re.S) or re.fullmatch(
            r"re:(.*?)(?::(\d+))?", locator, re.S)
        if m:
            try:
                rx = re.compile(m.group(1))
            except re.error as exc:
                out(f"===== {pattern}  -- bad regex /{m.group(1)}/: {exc}")
                out()
                return printed, empty + 1
            # A `:0` suffix is a real answer, not a missing one, so the suffix wins whenever the
            # group matched at all -- `"0" or context` is `"0"`, which is the point of testing the
            # string rather than the int.
            n = emit_matches(path, lines, rx, int(m.group(2) or context))
            printed += n
            if n:
                sweep_hits += 1
            elif not sweep:
                empty += 1
            continue

        span = parse_range(locator, len(lines))
        if span:
            printed += emit_span(path, lines, span[0], span[1], "lines")
            continue

        span = heading_span(lines, locator.strip("\"'"))
        if span:
            printed += emit_span(path, lines, span[0], span[1], f"§ {locator}")
            continue

        out(f"===== {rel(path)}  -- no heading matching {locator!r}, and it is not a line "
            f"range. Forms: 120-160, 120+30, @symbol, re:pattern (re:pattern:3 for context), "
            f"\"## Heading\".")
        out()
        empty += 1

    if sweep and not sweep_hits:
        out(f"===== {pattern}  -- {len(files)} file(s) matched the glob, none matched "
            f"{locator!r}")
        out()
        empty += 1
    return printed, empty


# ------------------------------------------------------------------------------- locate


def locate(names: list[str], scope: str | None) -> int:
    """Symbols in, `file:line  <the defining line>` out. No bodies: this is for anchors."""
    files = expand(scope) if scope else list(walk_repo())
    want = {n: [t.format(name=re.escape(n)) for t in DEFINITION] for n in names}
    compiled = {n: [re.compile(t) for t in ts] for n, ts in want.items()}
    found: dict[str, list[tuple[str, int, str]]] = {n: [] for n in names}

    for path in files:
        lines = read_lines(path)
        if lines is None:
            continue
        text = "\n".join(lines)
        for name in names:
            if name not in text:
                continue
            for rx in compiled[name]:
                hit = next((i for i, line in enumerate(lines) if rx.search(line)), None)
                if hit is not None:
                    found[name].append((rel(path), hit + 1, lines[hit].strip()))
                    break

    missing = 0
    for name in names:
        hits = found[name]
        if not hits:
            out(f"{name}: NOT FOUND")
            missing += 1
            continue
        for where, line, body in hits[:6]:
            out(f"{where}:{line}  {body[:110]}")
        if len(hits) > 6:
            out(f"  ... and {len(hits) - 6} more definition(s) of {name}")
    return missing


# --------------------------------------------------------------------------------- main


def outline(patterns: list[str], deep: bool) -> int:
    """One line per seam of a file: every `fn`, `struct`, `enum`, `trait`, `impl` and `mod`,
    with the line it starts on and how many lines it runs for.

    This is the answer to the measurement that says 56% of a session's read calls re-fetch a file
    it already opened -- 24.6 calls a session, over only 20.3 distinct files. The two hottest
    files in this repository are 5,044 and 5,720 lines, so reading one whole is not the fix and
    never will be; the fix is landing the *first* fetch on the right region. An outline of
    `lower/mod.rs` is about 4 KB against the file's 276 KB, and every line of it is a `:@name`
    target that lands exactly.

    Top-level seams only unless `--deep`: in that same file, 72 seams are top level and 218 are
    methods inside an `impl`, and printing all 290 costs 19 KB where the structure costs 5. A
    method is what `--locate <name>` answers in one line without printing any outline at all."""
    seen, nested = 0, 0
    for pattern in patterns:
        for path in expand(pattern):
            lines = read_lines(path)
            if lines is None:
                out(f"===== {pattern}  -- cannot read")
                continue
            rx = OUTLINE_PY if path.suffix == ".py" else OUTLINE
            hits = []
            for n, line in enumerate(lines, start=1):
                m = rx.match(line)
                if m and not line.lstrip().startswith(("//", "#", "*")):
                    hits.append((n, len(m.group("indent")), m.group("sig").strip()))
            shown = hits if deep else [h for h in hits if h[1] == 0]
            hidden = len(hits) - len(shown)
            out(f"===== {rel(path)}  {len(lines):,} lines, {len(hits)} seam(s)"
                + (f", {hidden} nested one(s) not shown" if hidden else ""))
            for i, (n, indent, sig) in enumerate(hits):
                if (n, indent, sig) not in shown:
                    continue
                nxt = hits[i + 1][0] if i + 1 < len(hits) else len(lines) + 1
                out(f"{n:>6}  {'  ' * min(indent // 4, 3)}{sig}   [{nxt - n} lines]")
            out()
            seen += 1
            nested += hidden
    if not seen:
        out("-- outline: nothing matched")
        return 1
    out("-- outline: every line above is a `:@name` target that lands on that seam exactly.")
    if nested:
        out(f"-- {nested} seam(s) nested inside an `impl` are not shown; `--deep` prints them, and")
        out("   `python tools/peek.py --locate <name>` finds one by name without printing any.")
    return 0


def note_reads(targets: list[str]) -> list[str]:
    """What this session's reading has cost so far, as advice lines for this call.

    A `peek.py` process cannot see the session it runs inside, so both counts live in a file.
    They share one read-modify-write of it on purpose: as two functions, whichever wrote second
    would drop the other's tally.

    *Re-fetches of one file* fire on every call past `REFETCH_NOTE_AT`, because the advice is to
    land the next fetch better and there is a next fetch every time it fires.

    *Single-target calls in a row* fire once per run of `SOLO_NOTE_AT` and then reset the run.
    The run is itself the thing being reported, so repeating the line on every call past four
    would be noise about a fact already stated.

    Both are advice and nothing else: never a refusal, never an exit code, and a missing or
    unwritable ledger is silently no advice rather than an error on a read."""
    now, key_now = time.time(), session_key()
    record = {"session": key_now, "at": now, "files": {}, "solo": 0}
    try:
        if LEDGER.exists():
            held = json.loads(LEDGER.read_text(encoding="utf-8"))
            fresh = (isinstance(held, dict)
                     and held.get("session") == key_now
                     and now - float(held.get("at", 0)) < LEDGER_IDLE_SECONDS)
            if fresh and isinstance(held.get("files"), dict):
                record["files"] = held["files"]
                record["solo"] = int(held.get("solo", 0) or 0)
    except (OSError, ValueError, TypeError):
        pass

    lines = []

    # A glob carries one target and reads a whole crate, so it is not a solo call in the sense
    # that costs a round trip -- only its spelling looks like one.
    globbed = any("*" in t or "?" in t for t in targets)
    record["solo"] = 0 if (len(targets) > 1 or globbed) else record["solo"] + 1
    if record["solo"] >= SOLO_NOTE_AT:
        lines.append(
            f"-- that is {record['solo']} calls in a row carrying one target. This tool takes as "
            "many as you have questions, and a session's wall clock is very nearly its round-trip "
            "count: `peek.py a.rs:120-160 b.rs:@sym c.md:\"## 4\"` is one call, not three.")
        record["solo"] = 0

    # Through `rule_target` first, or every `rule:x/y` in the run tallies as one file called
    # "rule" -- `split_target` splits on the first colon, and the token's is after four letters.
    paths = {split_target(rule_target(t) or t)[0] for t in targets}
    paths = {p for p in paths if "*" not in p and "?" not in p}
    hot = []
    tally = record["files"]
    for p in sorted(paths):
        name = p.replace("\\", "/")
        tally[name] = int(tally.get(name, 0)) + 1
        if tally[name] >= REFETCH_NOTE_AT:
            hot.append((name, tally[name]))
    try:
        LEDGER.parent.mkdir(parents=True, exist_ok=True)
        LEDGER.write_text(json.dumps(record), encoding="utf-8")
    except OSError:
        pass

    if hot:
        worst, count = max(hot, key=lambda x: x[1])
        lines.append(
            f"-- you have now fetched {worst} {count} times this session. "
            f"`python tools/peek.py --outline {worst}` prints its seams once, and every line of "
            "that is a `:@name` target that lands first time.")
    return lines


def target_forms() -> str:
    """The `Target forms` block of this file's docstring, verbatim. One home for the spelling."""
    lines = (__doc__ or "").splitlines()
    for i, line in enumerate(lines):
        if not line.startswith("Target forms"):
            continue
        block = [line]
        for nxt in lines[i + 1:]:
            if nxt and not nxt.startswith("    "):
                break
            block.append(nxt)
        return "\n".join(block).rstrip()
    return ""


class Parser(argparse.ArgumentParser):
    """argparse answers a bad flag with the usage line alone, and that line is where a session
    reaching for `grep`'s spelling gets stuck: it names `--window`, which applies to `@symbol` and
    to nothing else, and says nothing about how a `re:` target widens. So an error prints the
    target forms too -- the answer to the question the wrong flag was asking."""

    def error(self, message: str):
        self.print_usage(sys.stderr)
        sys.stderr.write(f"{self.prog}: error: {message}\n\n{target_forms()}\n")
        raise SystemExit(2)


def main() -> int:
    ap = Parser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("targets", nargs="*", metavar="TARGET",
                    help="path[:locator], one or many; a path may be a glob")
    ap.add_argument("--locate", nargs="+", metavar="SYMBOL",
                    help="symbols in, file:line out, no bodies")
    ap.add_argument("--outline", nargs="+", metavar="PATH",
                    help="one line per fn/struct/impl seam, with its span -- the map of a big file")
    ap.add_argument("--deep", action="store_true",
                    help="with --outline, include seams nested inside an impl")
    ap.add_argument("--in", dest="scope", metavar="GLOB",
                    help="restrict --locate to these files")
    ap.add_argument("--window", type=int, default=DEFAULT_WINDOW,
                    help=f"lines of body after an @symbol hit, and nothing else "
                         f"(default {DEFAULT_WINDOW}); a re: target takes --context")
    ap.add_argument("--context", "-C", type=int, default=0, metavar="N",
                    help="lines either side of every re: match (default 0); a target's own "
                         "re:pattern:N wins over this")
    ap.add_argument("--max-lines", type=int, default=DEFAULT_MAX_LINES,
                    help=f"refuse a whole file over this many lines (default {DEFAULT_MAX_LINES})")
    ap.add_argument("--quiet", action="store_true", help="omit the cost footer")
    opts = ap.parse_args()

    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    if opts.outline:
        return outline(opts.outline, opts.deep)

    if opts.locate:
        return 1 if locate(opts.locate, opts.scope) else 0

    if not opts.targets:
        ap.print_help()
        return 2

    total, empty = 0, 0
    for spec in opts.targets:
        printed, missed = peek_one(spec, opts.window, opts.max_lines, opts.context)
        total += printed
        empty += missed

    # The ledger is written whatever `--quiet` says: a call that does not count itself makes the
    # next call's advice wrong. Only the advice line is what `--quiet` suppresses.
    advice = note_reads(opts.targets)
    if not opts.quiet:
        out(f"-- peek: {len(opts.targets)} target(s) in one call, "
            f"{total:,} B (~{total / BYTES_PER_TOKEN:,.0f} tok)"
            + (f", {empty} produced nothing" if empty else ""))
        for line in advice:
            out(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
