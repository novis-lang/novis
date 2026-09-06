#!/usr/bin/env python3
"""The comment gate: no source comment carries a date, and none reads as a changelog.

`docs/agent/conventions.md` § *A code comment* is the rule; this is the half of it a machine can
judge. A comment states what the code does now, so when something was decided or changed is not a
property of the code -- `git blame` answers that exactly, and a comment answers it approximately and
then rots.

    python tools/prose.py             every finding, with its line; exit 0
    python tools/prose.py --check     the CI shape: quiet on success, exit 1 on a finding
    python tools/prose.py --changed   only files that differ from HEAD, plus the changelog warnings
    python tools/prose.py --list      the files this scans, one per line

WHAT FAILS, AND THE ESCAPE HATCH THAT IS NOT AN ALLOWLIST

A date in the prose of a comment fails. A date inside a backtick span does not, because there it is a
*value* -- a SQL literal, an epoch constant, the fixture a test is built on -- and the code is talking
about it rather than dating itself:

```
/// Days since `1970-01-01` as a civil date.               <- a value, and fine
// 2026-08-01 is a Saturday, so the first Monday differs.  <- a value, so backtick it
// Measured on 2026-09-06 over the conformance tree.       <- history; delete it
```

So there is no allowlist file to maintain and no per-line marker to learn: put a date that is data in
backticks, which is where every other literal in these comments already lives, and delete the rest.
A whole year on its own (`RFC 3339`, `PHP 8.4`, a copyright) is not a date and never fails.

Changelog wording -- "used to", "previously", "no longer" -- is a warning rather than a failure, and
only on lines a working tree has actually changed. Some of those phrasings describe the present
correctly ("the socket is no longer polled once drained"), so a gate on them would be a gate authors
fight; a warning on a line you just wrote is a question asked at the one moment you can answer it.

WHAT IT READS

Source only: the crates, the tools, the tests, the benches, the examples, the manifests and the CI
workflows. Not `docs/` -- a decision record is frozen on its date and says so -- not `CHANGELOG.md`,
which is generated, and not `website/`. The comment scanner below is shared with anything else that
needs to know which bytes of a source file are prose; `spans()` is the entry point.
"""

from __future__ import annotations

import argparse
import ast
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Where source lives. Walked in full; `SKIP_DIRS` prunes what is not ours or not source.
SCAN_DIRS = ("crates", "tools", "tests", "benches", "examples", "fuzz", "editors", "docker",
             ".github")
SKIP_DIRS = {"target", "node_modules", "out", ".vscode-test", "website", "php-src", ".git",
             "__pycache__", "snapshots"}
# Manifests and scripts at the root that carry prose worth the same rule.
ROOT_FILES = ("Cargo.toml", "nvs.toml", "deny.toml", "rustfmt.toml", "rust-toolchain.toml")

#: Suffix to comment family. A family is a scanner below, not a language: `.ts` and `.php` differ in
#: everything except which bytes are a comment.
FAMILY = {
    ".rs": "rust",
    ".nvs": "nvs", ".nvst": "nvst",
    ".py": "python",
    ".ts": "cstyle", ".mjs": "cstyle", ".php": "nvs",
    ".toml": "hash", ".yml": "hash", ".yaml": "hash", ".sh": "hash", ".conf": "hash",
    ".ini": "hash",
}
NAMED = {"Dockerfile": "hash", "commit-msg": "hash"}

ISO_DATE = r"(?:19|20)\d{2}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\d|3[01])"
MONTHS = ("January|February|March|April|May|June|July|August|September|October|November|December")
DATE = re.compile(
    rf"(?<![\w/.-])(?:{ISO_DATE}|(?:{MONTHS})\s+(?:19|20)\d{{2}}"
    rf"|\d{{1,2}}\s+(?:{MONTHS})\s+(?:19|20)\d{{2}})(?![\w/-])"
)

#: Wording that usually means a comment is describing its own past. A warning, never a failure --
#: each of these has an honest present-tense use, which is why the rule they serve is written for a
#: reader rather than a regex.
CHANGELOG_WORDS = re.compile(
    r"\b(?:used to(?: be)?|previously|formerly|until recently|has since|have since"
    r"|was renamed|renamed from|used to say|originally|before this change|prior to this"
    r"|this now|which now|no longer)\b", re.IGNORECASE)


# ---------------------------------------------------------------- the scanner

def spans(path, text):
    """Every comment in `text`, as `(start, end)` byte offsets into it, in order.

    A span covers the whole comment including its opener, so blanking every span leaves a file that
    is code alone -- which is what lets a rewrite of the prose be checked for having touched none of
    it. Returns an empty list for a suffix with no family.
    """
    family = FAMILY.get(path.suffix) or NAMED.get(path.name)
    if family is None:
        return []
    if family == "python":
        return _python(text)
    if family == "nvst":
        return _nvst(text)
    if family == "hash":
        return _scan(text, line=("#",), block=None, quotes="\"'", heredoc=False)
    if family == "rust":
        return _scan(text, line=("//",), block=("/*", "*/"), quotes="\"", raw=True, nested=True)
    if family == "cstyle":
        return _scan(text, line=("//",), block=("/*", "*/"), quotes="\"'`")
    return _scan(text, line=("//", "#"), block=("/*", "*/"), quotes="\"'", heredoc=True)


def _scan(text, line, block, quotes, raw=False, nested=False, heredoc=False):
    """One state machine over the families that spell a comment with punctuation.

    `raw` is Rust's `r#"..."#`, `nested` its `/* /* */ */`, `heredoc` the `<<<ID` body that PHP and
    Novis both have and that would otherwise look like a page of comments.
    """
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if c == "'" and raw and not _char_literal(text, i):
            i += 1                                     # a lifetime, not a literal
            continue
        if c in quotes:
            i = _string(text, i, c)
            continue
        if raw and c == "r" and (m := re.compile(r'r(#*)"').match(text, i)):
            close = '"' + m.group(1)
            end = text.find(close, m.end())
            i = n if end < 0 else end + len(close)
            continue
        if heredoc and (m := re.compile(r"<<<[ \t]*(['\"]?)(\w+)\1\r?\n").match(text, i)):
            end = re.compile(rf"^[ \t]*{m.group(2)}\b", re.MULTILINE).search(text, m.end())
            i = n if end is None else end.end()
            continue
        if block and text.startswith(block[0], i):
            depth, j = 1, i + len(block[0])
            while j < n and depth:
                if nested and text.startswith(block[0], j):
                    depth, j = depth + 1, j + len(block[0])
                elif text.startswith(block[1], j):
                    depth, j = depth - 1, j + len(block[1])
                else:
                    j += 1
            out.append((i, j))
            i = j
            continue
        opener = next((o for o in line if text.startswith(o, i)), None)
        if opener is not None and not (opener == "#" and text.startswith("#[", i)):
            end = text.find("\n", i)
            end = n if end < 0 else end
            out.append((i, end))
            i = end
            continue
        i += 1
    return out


def _string(text, i, quote):
    j = i + 1
    while j < len(text):
        if text[j] == "\\":
            j += 2
            continue
        if text[j] == quote or text[j] == "\n":
            return j + 1
        j += 1
    return len(text)


def _char_literal(text, i):
    return re.compile(r"'(?:\\.|[^\\'])'").match(text, i) is not None


def _python(text):
    """`#` comments, plus every docstring -- the module headers here carry more prose than the code.

    A file that will not parse falls back to its `#` comments, which is the honest answer: the
    docstrings of a broken file are not knowable and a syntax error is somebody else's finding.
    """
    out = _scan(text, line=("#",), block=None, quotes="\"'", heredoc=False)
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return out
    lines = text.splitlines(keepends=True)
    starts, total = [0], 0
    for ln in lines:
        total += len(ln)
        starts.append(total)
    for node in ast.walk(tree):
        if not isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            continue
        body = getattr(node, "body", None)
        if not body or not isinstance(body[0], ast.Expr):
            continue
        s = body[0].value
        if not (isinstance(s, ast.Constant) and isinstance(s.value, str)):
            continue
        out.append((starts[s.lineno - 1] + s.col_offset,
                    starts[s.end_lineno - 1] + s.end_col_offset))
    return sorted(out)


def _nvst(text):
    """A case file is sections; only two of them are prose a person writes.

    `--TEST--` is the description and is checked whole, `--FILE--` is Novis and is scanned for its
    comments. Everything else -- the expectation, the arguments, the environment -- is data the
    runner compares byte for byte, and a `//` in it is output, not a comment.
    """
    out = []
    for m in re.finditer(r"^--([A-Z_]+)--[ \t]*\r?\n", text, re.MULTILINE):
        nxt = re.compile(r"^--[A-Z_]+--[ \t]*\r?\n", re.MULTILINE).search(text, m.end())
        end = nxt.start() if nxt else len(text)
        if m.group(1) == "TEST":
            out.append((m.end(), end))
        elif m.group(1) == "FILE":
            body = text[m.end():end]
            out += [(m.end() + a, m.end() + b)
                    for a, b in _scan(body, line=("//", "#"), block=("/*", "*/"),
                                      quotes="\"'", heredoc=True)]
    return sorted(out)


# ---------------------------------------------------------------- the findings

def code_only(path, text):
    """`text` with every comment cut out and the remaining whitespace collapsed.

    Two files agreeing here differ in prose alone, which is the cheap proof that a rewrite of the
    comments rewrote only comments. Whitespace is normalised rather than preserved because a
    reflowed comment changes how many lines it occupies, and the question this answers is whether
    any *code* moved -- indentation is rustfmt's, not this projection's.
    """
    kept, at = [], 0
    for start, end in spans(path, text):
        kept.append(text[at:start])
        at = end
    kept.append(text[at:])
    return " ".join("".join(kept).split())


def prose_lines(path, text):
    """Each comment line as `(lineno, text-with-code-spans-removed)`.

    Backtick spans and fenced blocks come out because a date inside one is a value the comment is
    quoting. A fence is tracked across the lines of a single comment so a doc example survives.
    """
    fenced = False
    for start, end in spans(path, text):
        fenced = False
        first = text.count("\n", 0, start) + 1
        for offset, raw in enumerate(text[start:end].split("\n")):
            stripped = raw.strip().lstrip("/#*! \t")
            if stripped.startswith("```") or stripped.startswith("~~~"):
                fenced = not fenced
                continue
            if fenced:
                continue
            yield first + offset, re.sub(r"`[^`\n]*`", " ", raw)


def scan(path):
    """`(failures, warnings)` for one file, each a list of `(lineno, message, line)`."""
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return [], []
    fails, warns = [], []
    for lineno, line in prose_lines(path, text):
        for m in DATE.finditer(line):
            fails.append((lineno, f"date in a comment: {m.group(0)}", line.strip()))
    return fails, warns


def changed_warnings(path, added):
    """The changelog wording, on the lines this tree added. `added` is a set of line numbers."""
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return []
    out = []
    for lineno, line in prose_lines(path, text):
        if lineno in added and (m := CHANGELOG_WORDS.search(line)):
            out.append((lineno, f"reads as a changelog: \"{m.group(0)}\"", line.strip()))
    return out


# ---------------------------------------------------------------- the file set

def sources():
    seen = []
    for name in ROOT_FILES:
        if (ROOT / name).exists():
            seen.append(ROOT / name)
    for top in SCAN_DIRS:
        base = ROOT / top
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file() or any(p in SKIP_DIRS for p in path.parts):
                continue
            if path.suffix in FAMILY or path.name in NAMED:
                seen.append(path)
    return seen


def changed_files():
    """Paths that differ from HEAD, with the line numbers this tree added to each."""
    diff = subprocess.run(["git", "diff", "HEAD", "--unified=0"], cwd=ROOT,
                          capture_output=True, text=True, errors="replace").stdout
    out, path, lineno = {}, None, 0
    for line in diff.splitlines():
        if line.startswith("+++ b/"):
            path = ROOT / line[6:]
            out.setdefault(path, set())
        elif line.startswith("@@") and (m := re.search(r"\+(\d+)", line)):
            lineno = int(m.group(1))
        elif line.startswith("+") and path is not None:
            out[path].add(lineno)
            lineno += 1
    return {p: lines for p, lines in out.items()
            if p.suffix in FAMILY or p.name in NAMED}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0],
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="quiet on success, exit 1 on a finding")
    ap.add_argument("--changed", action="store_true", help="only what differs from HEAD")
    ap.add_argument("--list", action="store_true", help="the files this scans")
    opts = ap.parse_args()

    if opts.list:
        for path in sources():
            print(path.relative_to(ROOT).as_posix())
        return 0

    added = changed_files() if opts.changed else {}
    files = sorted(added) if opts.changed else sources()

    fails, warns = [], []
    for path in files:
        rel = path.relative_to(ROOT).as_posix()
        f, _ = scan(path)
        fails += [(rel, *row) for row in f]
        if opts.changed:
            warns += [(rel, *row) for row in changed_warnings(path, added[path])]

    for rel, lineno, msg, line in warns:
        print(f"warning {rel}:{lineno}  {msg}")
        print(f"        {line[:110]}")
    for rel, lineno, msg, line in fails:
        print(f"{rel}:{lineno}  {msg}")
        print(f"    {line[:110]}")

    if fails:
        print(f"\n{len(fails)} comment(s) carrying a date. A date that is a value belongs in "
              f"backticks; a date that is history belongs in `git log` and nowhere else.\n"
              f"docs/agent/conventions.md, 'A code comment', is the rule.")
        return 1
    if not opts.check:
        print(f"clean: {len(files)} file(s), no date in any comment.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
