#!/usr/bin/env python3
"""Name in `[context] modules` the source files a session actually edited.

    python tools/context-sync.py --since <sha>          # add what the range touched and is unnamed
    python tools/context-sync.py --since <sha> --dry-run
    python tools/context-sync.py --goal <toml> --since <sha>

`orient.py` builds a session's whole map out of `[context] modules` in `docs/agent/loop-goal.toml`,  # check-links:retired
and `playbook.py --goal` ranks the trap bullets against the same list. A goal is written before its
work is done, so the list names the files the author expected -- and a session that lands in one the
author did not expect gets no map line for it, no `//!` summary, and bullets ranked against a file
set that is missing the very file it is editing. Every later session of that goal pays the same
tax, because nothing on the loop's path corrects a manifest.

This corrects it, between sessions, from the one piece of evidence that cannot be argued with: what
the session's own commits touched. A path under `crates/*/src/` or `editors/*/src/` that no pattern
matches is appended with the module's own `//!` first sentence as its comment, which is the same
text `brief.py` prints for it and is true by construction.

**It only ever widens what a session may read**, which is why the driver is allowed to run it
unattended: an entry that is wrong costs one map line, an entry that is missing costs every session
of the goal a search. The two guards on that are `MAX_ADDED` and `MAX_TOTAL` below -- past either,
the sweep refuses and says so, because a manifest that grows without a person looking is the
unscoped pack the `[context]` block exists to replace.

Writes nothing but the `modules = [...]` list, and only when the result still parses as TOML. Prints
one line for the console; `--added <file>` also writes the added paths there, one per line, for
`tools/loop.py` to build its commit message from. Exit status is 0 whether or not anything was
added, and non-zero only when the goal file could not be read. Nothing here commits --
`tools/loop.py` stages and commits what this writes, so a hand-run leaves the change in the
working tree for you to look at.
"""

import argparse
import fnmatch
import re
import subprocess
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import brief  # noqa: E402  -- the module summaries, from the one place that reads them

ROOT = Path(__file__).resolve().parent.parent
GOAL = ROOT / "docs" / "agent" / "loop-goal.toml"

#: What `orient.py` can map, and therefore the only paths worth naming. A fixture or a doc is not a
#: module and is never added; a test module under `src/` matches, and `NEVER` takes it out.
MAPPABLE = (
    ("crates/*/src/**/*.rs", "crates/*/src/*.rs"),
    ("editors/*/src/**/*.ts", "editors/*/src/*.ts", "editors/*/src/**/*.tsx"),
)

#: Paths `MAPPABLE` matches that are never added. A crate's own root is where a `mod` line goes when
#: a module is added, so it comes back changed from work that was not in it; naming it by hand stays
#: right when the crate root *is* the work -- `nvs-diagnostics/src/lib.rs` is the code registry --
#: and that is a person's call, not a sweep's. A test module is edited beside the code it tests,
#: and that code is the entry a session needs. `fnmatch`'s `*` crosses `/`, so `*/tests/*` is a
#: `tests` directory at any depth.
NEVER = ("*/src/lib.rs", "*/tests/*", "*/tests.rs")

#: How wide a comment line may be, `docs/agent/doc-style.md`'s width -- a module summary runs long
#: and an entry nobody can read across is worse than the gap it closes.
WIDTH = 98

#: One sweep's ceiling. A session that touched a dozen unnamed modules is a goal whose manifest was
#: written for different work, and that is a person's call, not a sweep's.
MAX_ADDED = 4

#: The list's ceiling. `loop-authoring.md` § 2 asks for six to fifteen paths; past this the block
#: has stopped being a scope and the sweep says so rather than growing it further.
MAX_TOTAL = 18


def git(*args):
    try:
        r = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.SubprocessError):
        return ""
    return r.stdout if r.returncode == 0 else ""


def touched(since):
    """Every path the range wrote, as ROOT-relative posix. `since..HEAD`, the range the ledger and
    the goal row already count commits over."""
    out = git("diff", "--name-only", f"{since}..HEAD")
    return [ln.strip().replace("\\", "/") for ln in out.splitlines() if ln.strip()]


def mappable(path):
    if any(fnmatch.fnmatch(path, pat) for pat in NEVER):
        return False
    return any(fnmatch.fnmatch(path, pat) for group in MAPPABLE for pat in group)


def comment(summary):
    """The entry's why-line, wrapped, as the lines it occupies."""
    words, lines, line = summary.split(), [], "  #"
    for word in words:
        if line != "  #" and len(line) + 1 + len(word) > WIDTH:
            lines.append(line + "\n")
            line = "  #"
        line += " " + word
    lines.append(line + ("" if line.rstrip().endswith((".", ":", "?", "!")) else ".") + "\n")
    return lines


def named(path, patterns):
    """Whether an existing pattern already covers this path.

    The `/**` suffix is `orient.py`'s own rule and is repeated rather than shared, because the two
    tools reading one list must agree about what it means: a pattern naming a directory covers
    everything under it."""
    return any(fnmatch.fnmatch(path, pat) or fnmatch.fnmatch(path, pat.rstrip("/") + "/**")
               for pat in patterns)


def summary_of(path):
    """The module's own `//!` first sentence -- what `brief.py` prints for it."""
    text = brief.read(ROOT / path)
    if text is None:
        return ""
    return brief.first_sentence(brief.module_doc(text)) or ""


def block(text):
    """The `[context] modules` list as `(start, end)` offsets over the file's text.

    Found by hand rather than by round-tripping the TOML: a goal file is thousands of lines of
    frozen expected output with comments carrying half its meaning, and no writer preserves that.
    """
    ctx = re.search(r"^\[context\]\s*$", text, re.M)
    if not ctx:
        return None
    opened = re.search(r"^modules\s*=\s*\[\s*$", text[ctx.end():], re.M)
    if not opened:
        return None
    start = ctx.end() + opened.end()
    closed = re.search(r"^\]\s*$", text[start:], re.M)
    if not closed:
        return None
    return start, start + closed.start()


def sweep(goal_path, since, dry_run=False, wrote=None):
    """Add what the range touched and the manifest does not name. Returns the console line, and
    extends `wrote` with the paths it added to the file."""
    try:
        text = goal_path.read_text(encoding="utf-8")
        spec = tomllib.loads(text)
    except (OSError, tomllib.TOMLDecodeError) as e:
        raise SystemExit(f"context-sync: {goal_path} -- {e}")

    patterns = [str(p) for p in (spec.get("context") or {}).get("modules", [])]
    if not patterns:
        return ("context: the goal names no `[context] modules` at all, which is a goal to write "
                "rather than a gap to sweep -- nothing added")

    want = []
    for path in touched(since):
        if not mappable(path) or named(path, patterns) or path in want:
            continue
        if not (ROOT / path).is_file():  # a module the session deleted or moved
            continue
        want.append(path)
    if not want:
        return ""
    if len(want) > MAX_ADDED:
        return (f"context: the session edited {len(want)} modules `[context] modules` does not "
                f"name ({', '.join(want[:MAX_ADDED])}, ...) -- past {MAX_ADDED} in one session the "
                f"manifest was written for different work, so this is left for a person")
    if len(patterns) + len(want) > MAX_TOTAL:
        return (f"context: `[context] modules` is {len(patterns)} entries and naming "
                f"{', '.join(want)} would pass {MAX_TOTAL} -- left for a person to narrow")

    span = block(text)
    if span is None:
        return "context: `[context] modules` is not a plain `modules = [` list -- left alone"
    start, end = span

    added = []
    for path in want:
        summary = summary_of(path)
        added.extend(comment(summary) if summary else ["  # TODO: why a session opens this.\n"])
        added.append(f'  "{path}",\n')
    grown = text[:end] + "".join(added) + text[end:]
    try:
        tomllib.loads(grown)
    except tomllib.TOMLDecodeError as e:
        return f"context: naming {', '.join(want)} would not parse, so nothing was written -- {e}"
    if not dry_run:
        goal_path.write_text(grown, encoding="utf-8")
        if wrote is not None:
            wrote.extend(want)
    return f"context: `[context] modules` now names {', '.join(want)}"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0],
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--since", required=True, help="the sha the session started at")
    ap.add_argument("--goal", type=Path, default=GOAL, help="the goal file to widen")
    ap.add_argument("--dry-run", action="store_true", help="say what it would add, write nothing")
    ap.add_argument("--added", type=Path, help="write the paths it added here, one per line")
    opts = ap.parse_args()
    if not opts.since:
        return 0
    wrote = []
    line = sweep(opts.goal, opts.since, opts.dry_run, wrote)
    if line:
        print(line)
    if opts.added and wrote:
        opts.added.write_text("".join(f"{p}\n" for p in wrote), encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
