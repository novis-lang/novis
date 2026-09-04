#!/usr/bin/env python3
"""Report links in this repository that do not resolve, that name a file whose case does not match
the one on disk, or that are written in the wrong form for the file they sit in.

    python tools/check-links.py            # every tracked .md, .rs, .nvs and .nvst file
    python tools/check-links.py docs/adr   # only under these paths

**A gate: it exits non-zero on any finding, and CI's `docs` job runs it beside `adr.py --check`.** It
exists because a doc restructure moves dozens of relative links at once and a broken one is invisible
until someone follows it — that is a real defect with a ten-second fix, unlike a doc that runs a few
bytes long, which nothing here measures at all (docs/agent/doc-style.md § *Length targets*).

It was advisory, exiting 0 and run by nothing, for exactly as long as it took two renamed ADRs to
accumulate citations by their old filenames. A linter nothing runs is a linter that reports the same
findings for months; this one costs a second and needs no toolchain, so there is no reason for it to be
the kind that only answers when asked.

**A link's form depends on whether a renderer resolves it**, and this gate holds both halves of that
rule -- the one home for it is `docs/agent/conventions.md` § *Citing a document*:

*a markdown file* is rendered by GitHub and by the website, both of which resolve a link against the
    file's own location, so its links stay **relative** (`../adr/0067-core-db.md`).

*a source file* -- `.rs`, `.nvs`, `.nvst` -- is rendered by nothing. GitHub shows Rust as source, and
    rustdoc resolves a relative link against the generated HTML page rather than the module, where
    `../../../docs/` has never existed. So the only readers are people, agents and `grep`, and for
    those a link is **absolute from the repository root** (`/docs/adr/0067-core-db.md`): one spelling
    per target, and moving the file that holds it changes nothing. The relative form cost 465 dead
    links of 1,640 before it changed -- 442 with the wrong number of `../` and 23 naming a filename
    an ADR no longer had -- none of them reported by anything, because a `../` prefix encodes the
    *citing* file's depth and a moved module silently invalidates every one it carries.

`.py` is deliberately outside this gate: `tools/` emits markdown, so a link in a string literal there
is relative to the *generated* file and resolves from nowhere on disk.

Four kinds of finding:

*missing*  the target does not exist.

*case*     the target exists, but not with the spelling the link used. Windows and macOS resolve a
           path case-insensitively, so this is the failure that passes on the author's machine and 404s
           on GitHub and on every Linux checkout. It is the doc-side of the rule
           [ADR 0062](../docs/adr/0062-case-sensitivity-is-a-compiler-property.md) makes for `require`:
           a path is compared to the on-disk entry exactly.

*relative* a source file wrote `../docs/…` where the root-absolute form belongs.

*absolute* a markdown file wrote `/docs/…`, which resolves to the site root on GitHub and 404s.

Two link shapes in a source file are not paths and are skipped: a rustdoc intra-doc link naming an
item (`[the store](Cache::store)`, `[CLASS]`) has no `/` in it, and a link to a rustdoc page
(`../nvs_ir/ids/index.html`) is deliberately relative to the rendered HTML.

Fragments (`file.md#a-heading`) are checked as far as the file; the heading itself is not verified.

A file carrying the repository's `GENERATED FILE` marker in its head is skipped. `website/`'s ADR
mirror is written by `npm run sync:adrs` out of `docs/adr/`, and the transform rewrites every relative
link into a site route (`/docs/adr/0106/`) that resolves in Astro's router and never on disk. Checking
those cost 2,682 findings, all of them false, and left this gate red in CI's `docs` job for as long as
the site existed — while the links they are generated *from* are checked here in their source form,
which is the spelling a human actually edits.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent

# Inline links and images alike: `](target)` / `![alt](target)`. A target may not contain whitespace
# or `)`, which is true of every link in this repository and keeps the pattern from swallowing prose.
LINK_RE = re.compile(r"\]\(([^)\s]+)\)")

SKIP_SCHEMES = ("http://", "https://", "mailto:", "ftp://", "data:", "#")

# Rendered by a git host and by the website: links stay relative to the file. See the docstring.
DOC_EXTS = (".md",)
# Rendered by nothing: links are absolute from the repository root. See the docstring.
SOURCE_EXTS = (".rs", ".nvs", ".nvst")


def tracked_files(paths):
    """Every tracked file this gate reads, optionally filtered to the given path prefixes."""
    try:
        listing = subprocess.run(
            ["git", "ls-files"],
            cwd=ROOT,
            capture_output=True,
            encoding="utf-8",
            check=True,
        ).stdout.split("\n")
        files = [ROOT / line for line in listing if line]
    except (subprocess.CalledProcessError, OSError):
        # No git, or not a checkout: fall back to walking the tree, minus the build output.
        files = [p for p in ROOT.rglob("*") if p.is_file() and "target" not in p.parts]
    files = [f for f in files if f.suffix in DOC_EXTS + SOURCE_EXTS]

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

# Every generated page in this repository announces itself in its first few lines. See the docstring
# for why a generated page's links are not this gate's business.
GENERATED_MARKER = "GENERATED FILE"
GENERATED_HEAD_LINES = 15


def code_spans(line):
    """Character ranges covered by inline `code`. A link written inside one is an *example* —
    `[ADR NNNN](...)` in a sentence about how to write links — not a link to follow."""
    return [(m.start(), m.end()) for m in CODE_SPAN_RE.finditer(line)]


def is_generated(md_file):
    """True when the file announces itself as machine-written in its head. See the docstring."""
    try:
        text = md_file.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return False
    return any(GENERATED_MARKER in line for line in text.split("\n")[:GENERATED_HEAD_LINES])


def check(path):
    """Yields (line number, target, kind) for each finding in one file.

    Which form is correct depends on the file, and the docstring says why: a markdown file's links
    resolve from the file, a source file's from the repository root."""
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return
    source = path.suffix in SOURCE_EXTS
    fenced = False
    for lineno, line in enumerate(text.split("\n"), start=1):
        # A fenced block in a doc comment opens with `//! ```, which this deliberately does not
        # match: the fence rule is markdown's, and a source file's links are prose either way.
        if not source and FENCE_RE.match(line):
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
            if source:
                # Neither of these two names a file on disk. See the docstring.
                if "/" not in target or target.endswith(".html"):
                    continue
                if not target.startswith("/"):
                    yield lineno, raw, "relative"
                    continue
                base, target = ROOT, target.lstrip("/")
            else:
                if target.startswith("/"):
                    yield lineno, raw, "absolute"
                    continue
                base = path.parent
            if not (base / target).exists():
                yield lineno, raw, "missing"
            elif not case_exact(base, target):
                yield lineno, raw, "case"


def main():
    if any(a in ("-h", "--help") for a in sys.argv[1:]):
        sys.stdout.write(f"{__doc__}\n")
        return 0
    paths = [a for a in sys.argv[1:] if not a.startswith("-")]
    files = tracked_files(paths)

    findings = []
    generated = 0
    for path in sorted(files):
        if is_generated(path):
            generated += 1
            continue
        rel = path.relative_to(ROOT).as_posix()
        for lineno, target, kind in check(path):
            findings.append((rel, lineno, target, kind))

    for rel, lineno, target, kind in findings:
        sys.stdout.write(f"  {kind:<8} {rel}:{lineno}  ->  {target}\n")

    checked = len(files) - generated
    skipped = f", {generated} generated page(s) skipped" if generated else ""
    if findings:
        counts = Counter(f[3] for f in findings)
        tally = ", ".join(f"{n} {kind}" for kind, n in counts.most_common())
        sys.stdout.write(
            f"\n{len(findings)} finding(s) across {checked} file(s){skipped}: {tally}.\n"
            "A mis-cased link resolves on Windows and macOS and 404s everywhere else.\n"
            "A source file cites from the repository root (`/docs/…`), a markdown file from itself.\n"
        )
    else:
        sys.stdout.write(
            f"every link in {checked} file(s) resolves, with matching case and form{skipped}\n"
        )

    # A finding is a link that does not resolve, which is a defect rather than a
    # preference — so this is an exit status CI can act on. See the docstring.
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
