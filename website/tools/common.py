"""Shared helpers for the website tools.

Deliberately dependency-free: standard library only. The website subproject
already asks a developer to install a Node toolchain; asking for a Python
environment on top of that would be a second thing to get wrong, and everything
here is small enough not to need one.

Three ideas live in this file and are used by every tool:

* **Paths.** `REPO` is the repository root, `WEB` is `website/`. Nothing else
  computes them.
* **Frontmatter.** A deliberately small YAML subset — scalars and flat string
  lists — which is all this project's frontmatter contains. Reading is
  tolerant; writing only ever rewrites the one field asked for, textually, so
  a field this parser does not understand is never lost.
* **Keep regions.** `{/* keep */}…{/* /keep */}` in `.mdx`, or
  `<!-- keep -->…<!-- /keep -->` in `.md`, marks prose no tool and no agent may
  rewrite. Every generator that could overwrite an existing file calls
  `merge_keeps()` before writing.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent
REPO = WEB.parent

# --------------------------------------------------------------------------
# terminal
# --------------------------------------------------------------------------

# Windows consoles still default to a legacy code page, which turns every em
# dash in a message into a replacement character. The messages are the tool's
# whole interface, so make them survive.
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[union-attr]
    except (AttributeError, ValueError, OSError):
        pass

_COLOR = sys.stdout.isatty()


def _c(code: str, text: str) -> str:
    return f"\033[{code}m{text}\033[0m" if _COLOR else text


def ok(msg: str) -> None:
    print(f"  {_c('32', 'ok')}    {msg}")


def warn(msg: str) -> None:
    print(f"  {_c('33', 'warn')}  {msg}")


def fail(msg: str) -> None:
    print(f"  {_c('31', 'FAIL')}  {msg}")


def info(msg: str) -> None:
    print(f"  {_c('2', '·')}     {msg}")


def step(msg: str) -> None:
    print(f"\n{_c('1', msg)}")


# --------------------------------------------------------------------------
# git
# --------------------------------------------------------------------------


def blob_hash(path: Path) -> str | None:
    """The git blob hash of a file's *current on-disk content*.

    Not `git rev-parse HEAD:<path>` — that would report the committed content
    and miss an uncommitted edit, which is exactly the edit a writer needs to
    be told about.
    """
    if not path.is_file():
        return None
    try:
        out = subprocess.run(
            ["git", "hash-object", str(path)],
            cwd=REPO,
            capture_output=True,
            text=True,
            check=True,
        )
        return out.stdout.strip()[:12]
    except (OSError, subprocess.CalledProcessError):
        return None


def git_describe() -> str:
    """The version label for a local build: a tag if we are on one, else the branch."""
    for args in (["git", "describe", "--tags", "--exact-match"], ["git", "rev-parse", "--abbrev-ref", "HEAD"]):
        try:
            out = subprocess.run(args, cwd=REPO, capture_output=True, text=True, check=True)
            label = out.stdout.strip()
            if label and label != "HEAD":
                return label
        except (OSError, subprocess.CalledProcessError):
            continue
    return "main"


# --------------------------------------------------------------------------
# frontmatter
# --------------------------------------------------------------------------

_FM = re.compile(r"\A---\r?\n(.*?)\r?\n---\r?\n", re.DOTALL)


def split_frontmatter(text: str) -> tuple[str, str]:
    """Return `(frontmatter_block, body)`. The block excludes the `---` fences."""
    m = _FM.match(text)
    if not m:
        return "", text
    return m.group(1), text[m.end() :]


def parse_frontmatter(text: str) -> dict[str, object]:
    """Parse the YAML subset this project uses.

    Understands `key: scalar`, `key:` followed by an indented `- item` list, and
    quoted scalars. Anything more complex (Starlight's `hero:`, nested maps) is
    skipped rather than guessed at — the tools that use this only ever ask for
    `title`, `edit`, `sources` and `reviewed`.
    """
    block, _ = split_frontmatter(text)
    out: dict[str, object] = {}
    key: str | None = None
    items: list[str] = []

    def flush() -> None:
        nonlocal key, items
        if key is not None and items:
            out[key] = list(items)
        key, items = None, []

    for raw in block.splitlines():
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        if raw.startswith((" ", "\t", "-")):
            stripped = raw.strip()
            if key is not None and stripped.startswith("- "):
                items.append(_unquote(stripped[2:].strip()))
            continue
        flush()
        m = re.match(r"^([A-Za-z_][\w-]*)\s*:\s*(.*)$", raw)
        if not m:
            continue
        name, value = m.group(1), m.group(2).strip()
        if value == "":
            key = name
        else:
            out[name] = _unquote(value)
    flush()
    return out


def _unquote(v: str) -> str:
    if len(v) >= 2 and v[0] == v[-1] and v[0] in "\"'":
        inner = v[1:-1]
        return inner.replace('\\"', '"').replace("\\\\", "\\") if v[0] == '"' else inner
    return v


def yaml_quote(v: str) -> str:
    """Quote a scalar so it survives YAML. Titles here contain `:`, `—` and backticks."""
    return '"' + v.replace("\\", "\\\\").replace('"', '\\"') + '"'


def replace_frontmatter_list(text: str, key: str, values: list[str]) -> str:
    """Rewrite one list field in place, leaving every other byte of the file alone.

    Used by `site.py bless`. A full parse-and-reserialise would silently
    normalise fields this parser does not model, so it is not done.
    """
    block, body = split_frontmatter(text)
    if not block:
        raise ValueError("file has no frontmatter")

    lines = block.splitlines()
    start = next((i for i, l in enumerate(lines) if re.match(rf"^{key}\s*:", l)), None)
    rendered = [f"{key}:"] + [f"  - {v}" for v in values]

    if start is None:
        lines = lines + rendered
    else:
        end = start + 1
        while end < len(lines) and (lines[end].startswith((" ", "\t")) or lines[end].strip().startswith("- ")):
            end += 1
        lines[start:end] = rendered

    return "---\n" + "\n".join(lines) + "\n---\n" + body


# --------------------------------------------------------------------------
# keep regions
# --------------------------------------------------------------------------

# Two spellings, because the site has two content formats and each rejects the
# other's comment syntax: `.md` takes an HTML comment, `.mdx` takes a JSX one.
# Both mean exactly the same thing, and every tool accepts either.
#
#     <!-- keep -->  …  <!-- /keep -->        in .md
#     {/* keep */}   …  {/* /keep */}         in .mdx
#
# An optional note may follow the word: `{/* keep: reworded 2026-08-28 */}`.
KEEP_OPEN = r"(?:<!--|\{/\*)\s*keep\b[^\n]*?(?:-->|\*/\})"
KEEP_CLOSE = r"(?:<!--|\{/\*)\s*/keep\s*(?:-->|\*/\})"
KEEP = re.compile(KEEP_OPEN + r"\r?\n?(.*?)" + KEEP_CLOSE, re.DOTALL)
KEEP_OPEN_RE = re.compile(KEEP_OPEN)


def keep_regions(text: str) -> list[str]:
    """The bodies of every kept region, in order."""
    return [m.group(1) for m in KEEP.finditer(text)]


def merge_keeps(existing: str, generated: str) -> str:
    """Carry a file's kept regions into a freshly generated version of it.

    The Nth kept region of the old file replaces the Nth of the new one. If the
    new file has fewer regions than the old, the leftovers are appended in a
    clearly marked block rather than dropped — losing a human's paragraph is the
    one failure mode this whole mechanism exists to prevent, so the tool would
    rather leave a mess a person can see than be quietly tidy.
    """
    old = keep_regions(existing)
    if not old:
        return generated

    used = 0

    mdx = "{/*" in generated or "{/*" in existing

    def sub(m: re.Match[str]) -> str:
        nonlocal used
        if used < len(old):
            body = old[used]
            used += 1
            open_marker = KEEP_OPEN_RE.match(m.group(0))
            head = open_marker.group(0) if open_marker else m.group(0)
            close = "{/* /keep */}" if head.startswith("{") else "<!-- /keep -->"
            return f"{head}\n{body}{close}"
        return m.group(0)

    merged = KEEP.sub(sub, generated)

    if used < len(old):
        o, c = ("{/* keep */}", "{/* /keep */}") if mdx else ("<!-- keep -->", "<!-- /keep -->")
        orphans = "\n\n".join(f"{o}\n{b}{c}" for b in old[used:])
        note_open, note_close = ("{/*", "*/}") if mdx else ("<!--", "-->")
        merged += (
            f"\n\n{note_open} The regions below were kept from the previous version of\n"
            "     this page but no longer have a matching slot. Move them where\n"
            "     they belong, or delete them. Nothing automated will do it for\n"
            f"     you. {note_close}\n\n" + orphans + "\n"
        )

    return merged


# --------------------------------------------------------------------------
# markdown scanning
# --------------------------------------------------------------------------

_FENCE = re.compile(r"^(\s*)(`{3,}|~{3,})(.*)$")


def outside_code(text: str):
    """Yield `(line, in_code)` for each line, tracking fenced blocks.

    Every transform that rewrites markdown uses this rather than a bare regex,
    because a link-looking string inside a code fence is a code sample and must
    survive untouched.
    """
    fence: str | None = None
    for line in text.split("\n"):
        m = _FENCE.match(line)
        if m:
            marker = m.group(2)
            if fence is None:
                fence = marker
                yield line, True
                continue
            if marker[0] == fence[0] and len(marker) >= len(fence):
                fence = None
                yield line, True
                continue
        yield line, fence is not None


_INLINE_CODE = re.compile(r"`+[^`]*`+")


def map_outside_inline_code(line: str, fn) -> str:
    """Apply `fn` to the parts of a line that are not inside backticks."""
    out: list[str] = []
    last = 0
    for m in _INLINE_CODE.finditer(line):
        out.append(fn(line[last : m.start()]))
        out.append(m.group(0))
        last = m.end()
    out.append(fn(line[last:]))
    return "".join(out)
