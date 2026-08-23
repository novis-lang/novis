#!/usr/bin/env python3
"""Report markdown links in this repository that do not resolve, and links whose case does not
match the file on disk.

    python tools/check-links.py            # every tracked .md file
    python tools/check-links.py docs/adr   # only under these paths

**Advisory. It always exits 0, and nothing in this repository or in CI runs it.** It exists because a
doc restructure moves dozens of relative links at once and a broken one is invisible until someone
follows it — that is a real defect with a ten-second fix, unlike a doc that runs a few bytes long, which
nothing here measures at all (AGENTS.md § *Length targets, and why nothing enforces them*).

Two kinds of finding:

*missing*  the target does not exist.

*case*     the target exists, but not with the spelling the link used. Windows and macOS resolve a
           path case-insensitively, so this is the failure that passes on the author's machine and 404s
           on GitHub and on every Linux checkout. It is the doc-side of the rule
           [ADR 0062](../docs/adr/0062-case-sensitivity-is-a-compiler-property.md) makes for `require`:
           a path is compared to the on-disk entry exactly.

Fragments (`file.md#a-heading`) are checked as far as the file; the heading itself is not verified.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent

# Inline links and images alike: `](target)` / `![alt](target)`. A target may not contain whitespace
# or `)`, which is true of every link in this repository and keeps the pattern from swallowing prose.
LINK_RE = re.compile(r"\]\(([^)\s]+)\)")

SKIP_SCHEMES = ("http://", "https://", "mailto:", "ftp://", "data:", "#")


def tracked_markdown(paths):
    """Every tracked `.md` file, optionally filtered to the given path prefixes."""
    try:
        listing = subprocess.run(
            ["git", "ls-files", "*.md"],
            cwd=ROOT,
            capture_output=True,
            encoding="utf-8",
            check=True,
        ).stdout.split("\n")
        files = [ROOT / line for line in listing if line]
    except (subprocess.CalledProcessError, OSError):
        # No git, or not a checkout: fall back to walking the tree, minus the build output.
        files = [p for p in ROOT.rglob("*.md") if "target" not in p.parts]

    if not paths:
        return files
    prefixes = [(ROOT / p).resolve() for p in paths]
    return [f for f in files if any(str(f).startswith(str(pre)) for pre in prefixes)]


def case_exact(base, target):
    """True when every component of `target`, as literally written, matches its on-disk entry.

    `Path.exists()` answers the filesystem's question, and on Windows and macOS that question is
    case-insensitive -- which is exactly why a mis-cased link survives to a Linux reader.

    `Path.resolve()` is no help and is the trap here: on Windows it hands back the *canonical* spelling
    from the filesystem, so a mis-cased link comes out looking correct. So this walks the components as
    written, from a `base` whose own case is known good, and compares each one to `os.listdir`.
    """
    current = base
    for part in re.split(r"[/\\]", target):
        if part in ("", "."):
            continue
        if part == "..":
            current = current.parent
            if not str(current).startswith(str(ROOT)):
                return True  # walked out of the repository: not ours to judge
            continue
        try:
            entries = os.listdir(current)
        except OSError:
            return True
        if part not in entries:
            return False
        current = current / part
    return True


CODE_SPAN_RE = re.compile(r"`[^`]*`")
FENCE_RE = re.compile(r"^\s*(```|~~~)")


def code_spans(line):
    """Character ranges covered by inline `code`. A link written inside one is an *example* —
    `[ADR NNNN](...)` in a sentence about how to write links — not a link to follow."""
    return [(m.start(), m.end()) for m in CODE_SPAN_RE.finditer(line)]


def check(md_file):
    """Yields (line number, target, kind) for each finding in one file."""
    try:
        text = md_file.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return
    fenced = False
    for lineno, line in enumerate(text.split("\n"), start=1):
        if FENCE_RE.match(line):
            fenced = not fenced
            continue
        if fenced:
            continue
        spans = code_spans(line)
        for m in LINK_RE.finditer(line):
            if any(start <= m.start() < end for start, end in spans):
                continue
            raw = m.group(1)
            if raw.startswith(SKIP_SCHEMES):
                continue
            target = unquote(raw.split("#", 1)[0])
            if not target:
                continue  # a bare `#fragment`, handled above; nothing to resolve
            if not (md_file.parent / target).exists():
                yield lineno, raw, "missing"
            elif not case_exact(md_file.parent, target):
                yield lineno, raw, "case"


def main():
    paths = [a for a in sys.argv[1:] if not a.startswith("-")]
    files = tracked_markdown(paths)

    findings = []
    for md_file in sorted(files):
        rel = md_file.relative_to(ROOT).as_posix()
        for lineno, target, kind in check(md_file):
            findings.append((rel, lineno, target, kind))

    for rel, lineno, target, kind in findings:
        sys.stdout.write(f"  {kind:<8} {rel}:{lineno}  ->  {target}\n")

    checked = len(files)
    if findings:
        missing = sum(1 for f in findings if f[3] == "missing")
        miscased = len(findings) - missing
        sys.stdout.write(
            f"\n{len(findings)} finding(s) across {checked} file(s): "
            f"{missing} missing, {miscased} mis-cased.\n"
            "A mis-cased link resolves on Windows and macOS and 404s everywhere else.\n"
        )
    else:
        sys.stdout.write(f"every link in {checked} markdown file(s) resolves, with matching case\n")

    # Always 0: advisory by design. See this file's docstring.
    return 0


if __name__ == "__main__":
    sys.exit(main())
