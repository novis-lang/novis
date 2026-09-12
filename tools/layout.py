#!/usr/bin/env python3
"""CONTRIBUTING.md's repository-layout block, checked against the tree.

    python tools/layout.py            # the block as parsed, and what disagrees with disk
    python tools/layout.py --check    # quiet on success; exit non-zero on a finding
    python tools/layout.py --rows     # draft the rows the block is missing, read off disk

**A gate: it exits non-zero on any finding, and CI's `docs` job runs it beside `check-links.py`.**

It exists because that block is the one piece of prose in this repository that a *build* invalidates.
Adding a crate is a `Cargo.toml` and a `src/lib.rs`; nothing about that edit passes anywhere near
CONTRIBUTING.md, so the block silently stops describing the workspace and no test anywhere notices.
Measured on 2026-09-06, before this existed: the block named four crates as existing out of fifteen
that did, listed six crates (`nvs-regex`, `nvs-cache`, `nvs-fcgi` among them) that the plan had since
dissolved into others or deleted outright, named an `editors/` tree that is not on disk, carried a
milestone column against a schedule that is now a chain of numbered goals, and called `docs/spec/`
unwritten while holding three chapters. Every one of those is a fact about *disk*, which is why this
is a script and not a review habit.

So the rule the block is held to is: **every row names something that exists, and every crate, bench
package and tracked top-level directory has a row.** Nothing here reads a description — prose is a
person's job and a generator would write worse. What this owns is the *set* of rows, which is the half
that rots.

A row for something not yet built is therefore a finding, not an exemption. What the workspace grows
into belongs to `docs/implementation-plan.md`, which schedules it and is checked by `plan.py --check`;
a second copy of that schedule in CONTRIBUTING.md is the thing that went stale, and AGENTS.md's "every
fact has exactly one home" is the reason it does not come back.

Four kinds of finding:

*missing*    a row names a path that is not on disk.

*unlisted*   a crate, a bench package or a tracked top-level directory has no row. `--rows` drafts
             one for each, taking the description from the crate's own `//!` first sentence.

*unsafe*     the `[audited unsafe]` marker disagrees with the crate's own `[lints]`. The workspace
             sets `unsafe_code = "forbid"` and a crate that needs `unsafe` overrides it with its own
             `deny` plus site-level allows (Cargo.toml § *Lint policy*), so which crates carry the
             marker is derived and never remembered.

*shape*      the markers are gone, or what sits between them is not one fenced listing.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BLOCK = ROOT / "CONTRIBUTING.md"

BEGIN = "<!-- layout:begin"
END = "<!-- layout:end"
MARKER = "[audited unsafe]"

#: A crate opts out of the workspace's `unsafe_code = "forbid"` with this line in its own manifest.
OPT_OUT = re.compile(r'^\s*unsafe_code\s*=\s*"deny"', re.M)


def rel(path):
    return path.relative_to(ROOT).as_posix()


# ---------------------------------------------------------------------------
# What is on disk
# ---------------------------------------------------------------------------


def packages():
    """Every Cargo package under `crates/` and `benches/`, as repo-relative directory paths."""
    found = []
    for parent in ("crates", "benches"):
        base = ROOT / parent
        if not base.is_dir():
            continue
        for entry in sorted(base.iterdir()):
            if (entry / "Cargo.toml").is_file():
                found.append(rel(entry))
    return found


def tracked_dirs():
    """Every tracked top-level directory.

    `git ls-files` rather than a directory listing, because the answer has to match what a fresh
    clone holds: `target/` is on this machine and in nobody's checkout, and a block naming it would
    be wrong for every reader but the one who wrote it.
    """
    out = subprocess.run(
        ["git", "-C", str(ROOT), "ls-files"],
        capture_output=True, text=True, check=False,
    )
    if out.returncode != 0:
        return None
    dirs = set()
    for line in out.stdout.splitlines():
        head, sep, _ = line.partition("/")
        if sep:
            dirs.add(head)
    return sorted(dirs)


def audited_unsafe():
    """The packages carrying their own `unsafe_code = "deny"`, as repo-relative directory paths."""
    return [p for p in packages() if OPT_OUT.search((ROOT / p / "Cargo.toml").read_text("utf-8"))]


def first_sentence(package):
    """A package's `//!` opening sentence, flattened, for `--rows` to draft a description from."""
    for name in ("src/lib.rs", "src/main.rs"):
        source = ROOT / package / name
        if not source.is_file():
            continue
        doc = []
        for line in source.read_text("utf-8").splitlines():
            stripped = line.strip()
            if stripped.startswith("//!"):
                doc.append(stripped[3:].strip())
            elif doc:
                break
            elif stripped and not stripped.startswith("//"):
                break
        text = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", " ".join(doc)).strip()
        text = re.sub(r"\s+", " ", text)
        head = re.split(r"(?<=[.:])\s", text, maxsplit=1)[0]
        return head.rstrip(".:")
    return ""


# ---------------------------------------------------------------------------
# What the block says
# ---------------------------------------------------------------------------


def parse(text):
    """The block's rows as `(lineno, path, marked)`, or a *shape* finding as a string.

    A row is `<indent><name><spaces><description>`. An unindented name ending in `/` opens a group
    and every indented row under it hangs off that group; the nesting is one level deep, which is as
    deep as a listing stays readable.
    """
    lines = text.splitlines()
    starts = [i for i, line in enumerate(lines) if line.startswith(BEGIN)]
    ends = [i for i, line in enumerate(lines) if line.startswith(END)]
    if len(starts) != 1 or len(ends) != 1 or ends[0] < starts[0]:
        return f"expected exactly one {BEGIN} …> and one {END} …> after it"

    body = lines[starts[0] + 1:ends[0]]
    fences = [i for i, line in enumerate(body) if line.startswith("```")]
    if len(fences) != 2:
        return "what sits between the markers is not one fenced listing"

    rows, group = [], ""
    for offset, line in enumerate(body[fences[0] + 1:fences[1]]):
        if not line.strip():
            continue
        lineno = starts[0] + 2 + fences[0] + 1 + offset
        name = line.split()[0]
        if line[0].isspace():
            path = group + name
        else:
            group = name if name.endswith("/") else ""
            path = name
        rows.append((lineno, path.rstrip("/"), MARKER in line))
    return rows


# ---------------------------------------------------------------------------


def check():
    """Every finding, as `(kind, detail)` pairs, plus the packages that owe a row."""
    text = BLOCK.read_text("utf-8")
    rows = parse(text)
    if isinstance(rows, str):
        return [("shape", f"{rel(BLOCK)}: {rows}")], []

    findings, listed = [], {path for _, path, _ in rows}

    for lineno, path, _ in rows:
        if not (ROOT / path).exists():
            findings.append(("missing", f"{rel(BLOCK)}:{lineno}  ->  {path}"))

    owed = [p for p in packages() if p not in listed]
    for path in owed:
        findings.append(("unlisted", f"{path}  (a Cargo package with no row)"))

    dirs = tracked_dirs()
    if dirs is None:
        findings.append(("shape", "`git ls-files` failed, so top-level directories went unchecked"))
    else:
        for name in dirs:
            if name not in listed and not any(p.startswith(name + "/") for p in listed):
                findings.append(("unlisted", f"{name}/  (a tracked top-level directory with no row)"))

    expected = set(audited_unsafe())
    for lineno, path, marked in rows:
        if marked and path not in expected:
            findings.append(("unsafe", f"{rel(BLOCK)}:{lineno}  {path} carries {MARKER}, "
                                       "but its manifest does not override the workspace's forbid"))
    for path in sorted(expected - {p for _, p, m in rows if m}):
        if path in listed:
            findings.append(("unsafe", f"{path} declares its own `unsafe_code = \"deny\"` "
                                       f"and its row does not say {MARKER}"))

    return findings, owed


def main():
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--rows", action="store_true")
    parser.add_argument("-h", "--help", action="store_true")
    opts = parser.parse_args()

    if opts.help:
        sys.stdout.write(f"{__doc__}\n")
        return 0

    findings, owed = check()

    if opts.rows:
        if not owed:
            sys.stdout.write("nothing on disk owes a row\n")
        for path in owed:
            group, _, name = path.rpartition("/")
            sys.stdout.write(f"  {name:<16}  {first_sentence(path)}\n")
        return 1 if findings else 0

    for kind, detail in findings:
        sys.stdout.write(f"  {kind:<9} {detail}\n")

    if findings:
        sys.stdout.write(
            f"\n{len(findings)} finding(s) in {rel(BLOCK)}'s *Repository layout* listing.\n"
            "The block owns the set of rows; `python tools/layout.py --rows` drafts the missing ones.\n"
            "What the workspace has not built yet belongs to docs/implementation-plan.md, not here.\n"
        )
    elif not opts.check:
        rows = parse(BLOCK.read_text("utf-8"))
        sys.stdout.write(
            f"every one of the {len(rows)} rows in {rel(BLOCK)}'s layout listing is on disk, "
            f"and every crate, bench package and tracked top-level directory has one\n"
        )

    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
