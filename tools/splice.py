#!/usr/bin/env python3
"""Replace one exact block of text in a file with another.

AGENTS.md forbids carrying file content through a shell, and the Bash tool mangles a backslash
inside a heredoc -- which is every other line of Rust in this repository. So the blocks come
from files written with the Write tool, never from the command line.

    python tools/splice.py <target> --patch <patch-file>     # one file, preferred
    python tools/splice.py <target> <old-file> <new-file>    # two files
    python tools/splice.py <target> --patch <f> --dry-run    # does the anchor match?

A **patch file** holds both blocks, in conflict-marker form:

    <<<<<<< OLD
    the exact text to find
    =======
    the text to put there instead
    >>>>>>> NEW

One file means one Write call instead of two, which is the whole reason it exists: the loop's
sessions were reaching for `cat > file <<'EOF'` to avoid the second Write, and a heredoc is
exactly what eats the backslashes this repository is full of. A patch file may hold several
such blocks; they are applied in order, and any one of them failing to match leaves the target
completely untouched.

It refuses anything but exactly one match per block, so a stale or ambiguous anchor is an error
rather than a silent wrong edit, and it writes with newline='' so a file's existing line
endings survive untouched. On a failed match it says *where* the anchor stopped matching, which
is almost always a line of whitespace or a character that has since changed.
"""

from __future__ import annotations

import sys
from pathlib import Path

OPEN, MID, CLOSE = "<<<<<<< OLD", "=======", ">>>>>>> NEW"


def die(msg, code=1):
    sys.stdout.write(msg.rstrip("\n") + "\n")
    raise SystemExit(code)


def read(path):
    try:
        return Path(path).read_text(encoding="utf-8")
    except OSError as exc:
        die(f"splice.py: cannot read {path}: {exc}", 2)


def parse_patch(text, path):
    """[(old, new)] from conflict-marker blocks."""
    blocks, lines = [], text.split("\n")
    i = 0
    while i < len(lines):
        if lines[i].rstrip() != OPEN:
            if lines[i].strip() and not blocks:
                die(f"splice.py: {path} does not start with `{OPEN}` -- see this script's "
                    "docstring for the patch format", 2)
            i += 1
            continue
        try:
            mid = next(j for j in range(i + 1, len(lines)) if lines[j].rstrip() == MID)
            end = next(j for j in range(mid + 1, len(lines)) if lines[j].rstrip() == CLOSE)
        except StopIteration:
            die(f"splice.py: a `{OPEN}` block in {path} is missing its `{MID}` or `{CLOSE}`", 2)
        blocks.append(("\n".join(lines[i + 1 : mid]), "\n".join(lines[mid + 1 : end])))
        i = end + 1
    if not blocks:
        die(f"splice.py: no `{OPEN}` block in {path}", 2)
    return blocks


def where_it_diverges(src, old):
    """The longest prefix of `old` that IS in `src`, and what follows it on both sides.
    A stale anchor is the usual failure, and this says which line went stale."""
    lo, hi = 0, len(old)
    while lo < hi:
        mid = (lo + hi + 1) // 2
        if old[:mid] in src:
            lo = mid
        else:
            hi = mid - 1
    if lo == 0:
        return "  not even the first line of the anchor appears in the target."
    at = src.index(old[:lo])
    line = src[:at].count("\n") + old[:lo].count("\n") + 1
    got = src[at + lo :].split("\n", 1)[0]
    want = old[lo:].split("\n", 1)[0]
    return (
        f"  the anchor matches for its first {lo} character(s), up to target line {line}.\n"
        f"    the file has: {got[:70]!r}\n"
        f"    the anchor wants: {want[:70]!r}"
    )


def main(argv):
    if len(argv) < 2:
        die(__doc__.strip().split("\n\n")[1], 2)

    target = argv[0]
    rest = [a for a in argv[1:] if a != "--dry-run"]
    dry = "--dry-run" in argv

    if rest[:1] == ["--patch"]:
        if len(rest) != 2:
            die("splice.py: --patch takes exactly one file", 2)
        blocks = parse_patch(read(rest[1]), rest[1])
    elif len(rest) == 2:
        blocks = [(read(rest[0]), read(rest[1]))]
    else:
        die("usage: splice.py <target> --patch <patch-file>\n"
            "       splice.py <target> <old-file> <new-file>", 2)

    src = read(target)

    # Check every block before writing anything: a patch that half-applies is worse than one
    # that does not apply at all.
    probe = src
    for n, (old, new) in enumerate(blocks, start=1):
        if not old:
            die(f"splice.py: block {n} has an empty OLD section -- refusing", 2)
        count = probe.count(old)
        if count != 1:
            label = f"block {n} of {len(blocks)}" if len(blocks) > 1 else "the anchor"
            if count == 0:
                die(f"splice.py: {label} does not appear in {target}.\n"
                    + where_it_diverges(probe, old))
            lines = [probe[:m].count("\n") + 1
                     for m in range(len(probe)) if probe.startswith(old, m)]
            die(f"splice.py: {label} appears {count} times in {target} "
                f"(lines {', '.join(map(str, lines))}) -- make it unique")
        probe = probe.replace(old, new, 1)

    if dry:
        sys.stdout.write(
            f"splice.py: dry run -- all {len(blocks)} block(s) match exactly once in {target}\n"
        )
        return 0

    Path(target).write_text(probe, encoding="utf-8", newline="")
    sys.stdout.write(
        f"splice.py: {len(blocks)} block(s) spliced into {target}\n"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
