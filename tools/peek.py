#!/usr/bin/env python3
"""Read many places in many files in ONE tool call.

Measured over a full loop run: **0 of 3,647** tool-call messages carried more than one call,
against an AGENTS.md rule that asks for exactly that. Thirty-nine sessions read that rule and
none applied it, including runs of 52 and 57 consecutive `grep`/`sed` calls. A later 33-session run
measured the same 1.00 calls per message. That experiment is finished: a rule the model must
remember and choose loses to a tool it can just call. `tools/splice.py` is the same move for the
write side, where runs of consecutive `Edit` calls were costing 10.3 turns a session.

So this is batching as a *tool*. One call, N targets, N answers -- and the batching happens
whether or not anyone thought about it.

    python tools/peek.py crates/mwl-ir/src/lower/expr.rs:3065-3120 \\
                         crates/mwl-types/src/expr/members.rs:@lower_shape_property_access \\
                         docs/adr/0036-shapes.md:"## 4" \\
                         "crates/mwl-runtime/src/*.rs:/slot_get/"

Target forms, all of them `path` followed by `:` and a locator:

    path                  the whole file (refused over --max-lines, which says so and stops)
    path:120-160          those lines
    path:120+30           30 lines starting at 120
    path:@name            the line that *defines* `name`, plus --window lines of context
    path:re:regex         every matching line, `grep -n` style
    path:re:regex:3       every matching line with 3 lines of context either side
    path:/regex/          the same, in the familiar spelling -- but Git Bash on Windows
                          rewrites a leading `/` into a Win32 path before this tool sees it,
                          so prefer `re:` there
    path:"## Heading"     a markdown heading and its body, to the next same-or-higher heading

`path` may be a glob (`crates/**/*.rs`), in which case the locator runs against every match --
which is how you sweep a regex across a crate without a second call.

    python tools/peek.py --locate mwl_object_slot_get SlotSet ClassDesc
    python tools/peek.py --outline crates/mwl-ir/src/lower/mod.rs

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
TEXT_SUFFIXES = {".rs", ".py", ".md", ".toml", ".mwl", ".mwlt", ".txt", ".json", ".yml",
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

#: Python's seams, kept separate because `class Adder` is also MWL, and this repository's Rust is
#: full of MWL fixtures in string literals -- matching them turned a `lower/mod.rs` outline into
#: a list of test classes.
OUTLINE_PY = re.compile(r"^(?P<indent>\s*)(?P<sig>(?:async\s+)?(?:def|class)\s+\w+)")

#: Where this session's fetches are remembered, so a repeat can be reported. One file per
#: process tree is not possible -- `peek.py` is a fresh process every call -- so it is one file
#: per day under `.agent-tmp/`, which is gitignored and swept with the rest of it.
LEDGER = ROOT / ".agent-tmp" / "peek-ledger.json"

#: Fetches of one file, in one session, past which the footer says so. Three is where reading
#: the seams first (`--outline`) starts to beat guessing at another region.
REFETCH_NOTE_AT = 3

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
    same section -- the same normalisation orient.py uses, for the same reason."""
    def norm(t: str) -> str:
        t = re.sub(r"[`*_#]", "", t).strip().lower()
        t = re.sub(r"^(\d+)\s*[.)]?\s*", r"\1 ", t)
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


def peek_one(spec: str, window: int, max_lines: int) -> tuple[int, int]:
    """One target -> (bytes printed, targets that produced nothing)."""
    pattern, locator = split_target(spec)
    files = expand(pattern)
    if not files:
        out(f"===== {pattern}  -- NO SUCH FILE")
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
            n = emit_matches(path, lines, rx, int(m.group(2) or 0))
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
            f"range. Forms: 120-160, 120+30, @symbol, /regex/, \"## Heading\".")
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


def note_refetch(targets: list[str]) -> str | None:
    """One line when this session has now fetched the same file `REFETCH_NOTE_AT` times.

    A `peek.py` process cannot see the session it runs inside, so the count lives in a file. It
    is advice and nothing else: it never refuses, never changes an exit code, and a missing or
    unwritable ledger is silently no advice at all rather than an error on a read."""
    paths = {split_target(t)[0] for t in targets}
    paths = {p for p in paths if "*" not in p and "?" not in p}
    if not paths:
        return None
    now, key_now = time.time(), session_key()
    record = {"session": key_now, "at": now, "files": {}}
    try:
        if LEDGER.exists():
            held = json.loads(LEDGER.read_text(encoding="utf-8"))
            fresh = (isinstance(held, dict)
                     and held.get("session") == key_now
                     and now - float(held.get("at", 0)) < LEDGER_IDLE_SECONDS)
            if fresh and isinstance(held.get("files"), dict):
                record["files"] = held["files"]
    except (OSError, ValueError, TypeError):
        pass

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

    if not hot:
        return None
    worst, count = max(hot, key=lambda x: x[1])
    return (f"-- you have now fetched {worst} {count} times this session. "
            f"`python tools/peek.py --outline {worst}` prints its seams once, and every line of "
            "that is a `:@name` target that lands first time.")


def main() -> int:
    ap = argparse.ArgumentParser(
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
                    help=f"lines of body after an @symbol hit (default {DEFAULT_WINDOW})")
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
        printed, missed = peek_one(spec, opts.window, opts.max_lines)
        total += printed
        empty += missed

    # The ledger is written whatever `--quiet` says: a call that does not count itself makes the
    # next call's advice wrong. Only the advice line is what `--quiet` suppresses.
    advice = note_refetch(opts.targets)
    if not opts.quiet:
        out(f"-- peek: {len(opts.targets)} target(s) in one call, "
            f"{total:,} B (~{total / BYTES_PER_TOKEN:,.0f} tok)"
            + (f", {empty} produced nothing" if empty else ""))
        if advice:
            out(advice)
    return 0


if __name__ == "__main__":
    sys.exit(main())
