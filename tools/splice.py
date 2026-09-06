#!/usr/bin/env python3
"""Replace exact blocks of text, across as many files as one edit touches, in one call.

AGENTS.md forbids carrying file content through a shell, and the Bash tool mangles a backslash
inside a heredoc -- which is every other line of Rust in this repository. So the blocks come
from files written with the Write tool, never from the command line.

    python tools/splice.py --patch <patch-file>              # any number of files, preferred
    python tools/splice.py <target> --patch <patch-file>     # one file, targets named on argv
    python tools/splice.py <target> <old-file> <new-file>    # two files
    python tools/splice.py --patch <f> --dry-run             # do the anchors match?
    python tools/splice.py --help                            # this, down to the patch format

A **patch file** holds the blocks, in conflict-marker form, each under the file it edits:

    --- crates/nvs-ir/src/lower/expr.rs
    <<<<<<< OLD
    the exact text to find
    =======
    the text to put there instead
    >>>>>>> NEW
    <<<<<<< OLD
    a second block in the same file
    =======
    its replacement
    >>>>>>> NEW
    --- crates/nvs-types/src/expr/mod.rs
    <<<<<<< OLD
    a block in a different file
    =======
    its replacement
    >>>>>>> NEW

A `--- <path>` line applies to every block under it until the next one. With a `<target>` on the
command line the headers may be left out entirely, which is the older one-file form and still
works. **Either way the whole patch applies or none of it does** -- every block in every file is
matched before a single byte is written, so a stale anchor in the last file cannot leave the first
one half-edited.

## Why this exists, and why it takes many files

Two different costs, and the multi-file form is about the second.

The first is the heredoc: the loop's sessions were reaching for `cat > file <<'EOF'` to avoid a
second Write call, and a heredoc is exactly what eats the backslashes this repository is full of.

The second is **turns**. A session's wall clock is very nearly its turn count times a constant --
measured over a 33-session run, time-to-first-token was ~80% of a turn and did not depend on what
the turn did -- and 25 of a session's ~95 calls are `Edit`/`Write`. Those arrive in *runs*: 10.3
turns a session were spent on `Edit` calls issued back to back with nothing read between them, in
runs of up to 17. Every one of those runs is a single decision the model already made, spent one
round trip at a time.

`docs/agent/commands.md` records what happened when the fix was a *rule* -- "issue independent
calls in the same message" lost 3,647 times out of 3,647, in sessions that had the rule in
context. `peek.py` is what actually took that saving on the read side, by being a tool instead of
an instruction. This is the same move for the write side: one Write of the patch file, one call to
apply it, however many files and blocks the edit spans. A run of three or more edits is cheaper
here; one or two are cheaper as plain `Edit` calls, and that is the whole rule for choosing.

It refuses anything but exactly one match per block, so a stale or ambiguous anchor is an error
rather than a silent wrong edit, and it writes with newline='' so a file's existing line endings
survive untouched. On a failed match it says *where* the anchor stopped matching, which is almost
always a line of whitespace or a character that has since changed.
"""

from __future__ import annotations

import sys
from pathlib import Path

OPEN, MID, CLOSE = "<<<<<<< OLD", "=======", ">>>>>>> NEW"
FILE_MARK = "--- "


def die(msg, code=1):
    sys.stdout.write(msg.rstrip("\n") + "\n")
    raise SystemExit(code)


def help_text(full):
    """`--help` prints everything above the rationale; a misuse prints the invocation forms alone.

    Both are sliced out of the module docstring rather than written a second time here, because a
    second copy is what went wrong: `--dry-run` was documented at the top of this file and in no
    string `--help` could reach, so two consecutive optimization passes hand-verified the same
    false positive. The full form stops at the rationale heading -- a session reaching for `--help`
    wants the patch format, not why the tool exists.
    """
    doc = __doc__.strip()
    if full:
        return doc.split("## Why this exists", 1)[0].rstrip()
    forms = [p for p in doc.split("\n\n") if p.lstrip().startswith("python tools/splice.py")]
    return (forms[0] if forms else doc) + "\n\n`--help` prints the patch format in full."


def read(path):
    try:
        return Path(path).read_text(encoding="utf-8")
    except OSError as exc:
        die(f"splice.py: cannot read {path}: {exc}", 2)


def parse_patch(text, path, default_target=None):
    """[(target, old, new)] from the patch's blocks, in file order then block order.

    A `--- <path>` line switches which file the blocks below it edit. Without any such line the
    patch is the older single-file form and every block goes to `default_target`; mixing the two
    -- a `<target>` on argv *and* headers in the file -- is refused rather than guessed at, since
    the two say different things about where a block lands."""
    blocks, lines = [], text.split("\n")
    target, saw_header = default_target, False
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith(FILE_MARK) and line.rstrip() != MID:
            named = line[len(FILE_MARK):].strip()
            if not named:
                die(f"splice.py: {path} line {i + 1} is a bare `{FILE_MARK.strip()}` "
                    "with no path after it", 2)
            if default_target is not None:
                die(f"splice.py: {path} names files with `{FILE_MARK}{named}` and a target was "
                    "also given on the command line. Use one or the other.", 2)
            target, saw_header = named, True
            i += 1
            continue
        if line.rstrip() != OPEN:
            if line.strip() and not blocks and not saw_header:
                die(f"splice.py: {path} does not start with `{OPEN}` or `{FILE_MARK.strip()} "
                    "<path>` -- see this script's docstring for the patch format", 2)
            i += 1
            continue
        if target is None:
            die(f"splice.py: {path} line {i + 1} opens a block before any "
                f"`{FILE_MARK}<path>` line, and no target was given on the command line", 2)
        try:
            mid = next(j for j in range(i + 1, len(lines)) if lines[j].rstrip() == MID)
            end = next(j for j in range(mid + 1, len(lines)) if lines[j].rstrip() == CLOSE)
        except StopIteration:
            die(f"splice.py: a `{OPEN}` block in {path} is missing its `{MID}` or `{CLOSE}`", 2)
        blocks.append((target, "\n".join(lines[i + 1 : mid]), "\n".join(lines[mid + 1 : end])))
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
    # Every message this tool prints quotes the file it failed on, and this repository's prose is
    # full of `§` and em dashes. Without this, a failed match on such a line dies inside the
    # console codec instead of saying which anchor went stale.
    try:
        sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    except AttributeError:
        pass

    # `--help` exits 0 on purpose: `loop.py`'s `tools_still_load` probes every changed
    # `tools/*.py` with it after an optimization pass, and reads a non-zero status as a broken
    # script it must roll the pass back over.
    if any(a in ("-h", "--help") for a in argv):
        die(help_text(True), 0)
    if not argv:
        die(help_text(False), 2)

    dry = "--dry-run" in argv
    args = [a for a in argv if a != "--dry-run"]

    if args[:1] == ["--patch"]:
        if len(args) != 2:
            die("splice.py: --patch takes exactly one file", 2)
        blocks = parse_patch(read(args[1]), args[1])
    elif len(args) >= 2 and args[1] == "--patch":
        if len(args) != 3:
            die("splice.py: --patch takes exactly one file", 2)
        blocks = parse_patch(read(args[2]), args[2], default_target=args[0])
    elif len(args) == 3:
        blocks = [(args[0], read(args[1]), read(args[2]))]
    else:
        die(help_text(False), 2)

    # Check every block in every file before writing anything: a patch that half-applies is
    # worse than one that does not apply at all, and with several targets "half" now means
    # "some files edited and some not", which no session could unpick cheaply.
    staged: dict[str, str] = {}
    order: list[str] = []
    for n, (target, old, new) in enumerate(blocks, start=1):
        if target not in staged:
            staged[target] = read(target)
            order.append(target)
        if not old:
            die(f"splice.py: block {n} has an empty OLD section -- refusing", 2)
        probe = staged[target]
        count = probe.count(old)
        if count != 1:
            label = f"block {n} of {len(blocks)}" if len(blocks) > 1 else "the anchor"
            if count == 0:
                die(f"splice.py: {label} does not appear in {target}.\n"
                    + where_it_diverges(probe, old))
            at = [probe[:m].count("\n") + 1
                  for m in range(len(probe)) if probe.startswith(old, m)]
            die(f"splice.py: {label} appears {count} times in {target} "
                f"(lines {', '.join(map(str, at))}) -- make it unique")
        staged[target] = probe.replace(old, new, 1)

    if dry:
        sys.stdout.write(
            f"splice.py: dry run -- all {len(blocks)} block(s) match exactly once "
            f"across {len(order)} file(s)\n"
        )
        return 0

    for target in order:
        Path(target).write_text(staged[target], encoding="utf-8", newline="")
    per = {t: sum(1 for b in blocks if b[0] == t) for t in order}
    sys.stdout.write(
        f"splice.py: {len(blocks)} block(s) spliced into {len(order)} file(s)\n"
        + "".join(f"  {n:>3} block(s)  {t}\n" for t, n in per.items())
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
