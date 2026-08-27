"""Mirror `docs/adr/` into the public site.

The ADRs are the project's reasoning, already written, already cross-checked by
`tools/adr.py`, already holding the rule that every other document defers to.
Restating them in public prose would create a second home for every decision,
which is the one thing this whole website is designed not to do. So they are
published as they are, and the authored documentation *links* to them instead
of explaining them again.

Output is `website/src/content/docs/design/` and it is gitignored. Nothing here
is ever hand-edited: the file header of every generated page says so, and says
which source file to edit instead.

Six transforms, and no others — this is a change of shape, not of content:

1. The H1 becomes Starlight frontmatter, so the page renders one title.
2. A one-line description is lifted from the `In short` block for search
   results and social cards.
3. Relative links between ADRs are rewritten to site URLs; links to repository
   files that are *not* published resolve to the source host.
4. Bare `<` outside code becomes `&lt;`, so `array<T>` in prose survives
   markdown's HTML parsing.
5. ```` ```php ```` fences become ```` ```nvs ````. The samples were always
   Novis; the fence said `php` because no Novis grammar existed. Publishing
   them tagged `php` would imply the compatibility ADR 0080 forbids claiming.
6. A short note is prepended saying the page is a mirror and where its source
   lives.
"""

from __future__ import annotations

import posixpath
import re
from pathlib import Path

from common import REPO, WEB, map_outside_inline_code, ok, outside_code, warn, yaml_quote

ADR_DIR = REPO / "docs" / "adr"
OUT_DIR = WEB / "src" / "content" / "docs" / "design"

# Overridden by site.py from the environment so a mirror built for a fork or a
# different host points at the right place.
REPO_URL = "https://github.com/BrainFooLong/novis"
REPO_REF = "main"

ADR_FILE = re.compile(r"^(\d{4})-([a-z0-9-]+)\.md$")
LINK = re.compile(r"\[([^\]]*)\]\(([^)\s]+)(\s+\"[^\"]*\")?\)")
H1 = re.compile(r"^#\s+(.*)$", re.MULTILINE)


def _rel(from_kind: str, to: str) -> str:
    """A relative URL from one mirrored page to another.

    Relative rather than root-absolute on purpose: every version of this site is
    built under a different base path (`/`, `/main/`, `/v0.3/`), and a
    root-absolute link would leave the version the reader chose.
    """
    up = {"adr": "../../", "index": "", "leaf": "../"}[from_kind]
    return up + to


def _link_target(target: str, from_kind: str) -> str:
    if target.startswith(("http://", "https://", "#", "mailto:")):
        return target

    path, _, anchor = target.partition("#")
    anchor = f"#{anchor}" if anchor else ""

    m = ADR_FILE.match(path)
    if m:
        slug = path[:-3]
        return _rel(from_kind, "adr/" + slug + "/") + anchor
    if path == "README.md":
        return (_rel(from_kind, "") or "./") + anchor
    if path in ("ground-rules.md", "divergences.md"):
        return _rel(from_kind, path[:-3] + "/") + anchor

    # Anything else is a repository file this site does not publish: the spec
    # tree, the plan, `crates/`, `docs/agent/`. Send the reader to the source.
    resolved = posixpath.normpath(posixpath.join("docs/adr", path)).lstrip("./")
    return f"{REPO_URL}/blob/{REPO_REF}/{resolved}{anchor}"


def _rewrite_links(text: str, from_kind: str) -> str:
    out: list[str] = []
    for line, in_code in outside_code(text):
        if in_code:
            out.append(line)
            continue
        out.append(
            map_outside_inline_code(
                line,
                lambda seg: LINK.sub(
                    lambda m: f"[{m.group(1)}]({_link_target(m.group(2), from_kind)}{m.group(3) or ''})",
                    seg,
                ),
            )
        )
    return "\n".join(out)


def _escape_angles(text: str) -> str:
    """`array<T>` in prose is a type, not an HTML tag."""
    out: list[str] = []
    for line, in_code in outside_code(text):
        if in_code:
            out.append(line)
            continue
        out.append(map_outside_inline_code(line, lambda seg: seg.replace("<", "&lt;")))
    return "\n".join(out)


def _retag_fences(text: str) -> str:
    return re.sub(r"^(\s*)(`{3,})php\s*$", r"\1\2nvs", text, flags=re.MULTILINE)


def _description(body: str) -> str:
    """One sentence from the `In short` block, for search results and OG cards."""
    m = re.search(r"^>\s*\*\*In short:\*\*\s*(.*(?:\n>.*)*)", body, re.MULTILINE)
    if not m:
        return ""
    quote = re.sub(r"\n>\s?", " ", m.group(1))
    plain = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", quote)
    plain = re.sub(r"[*`_]", "", plain)
    plain = re.sub(r"\s+", " ", plain).strip()
    sentence = re.split(r"(?<=[.;])\s", plain)[0]
    # An `In short` often opens mid-sentence ("exactly two interfaces exist"),
    # because the `**In short:**` label is its subject. Standing alone as a
    # description it needs a capital.
    if sentence and sentence[0].islower():
        sentence = sentence[0].upper() + sentence[1:]
    if len(sentence) > 170:
        sentence = sentence[:167].rsplit(" ", 1)[0] + "…"
    return sentence


def _mirror_note(source_rel: str, extra: str = "") -> str:
    href = f"{REPO_URL}/blob/{REPO_REF}/{source_rel}"
    return (
        '<div class="nv-mirror-note">\n'
        "  <div>\n"
        f"    <strong>Mirrored from the repository.</strong> This page is "
        f'<code>{source_rel}</code>, published as written — it is the decision '
        "itself, not a summary of one. The guides elsewhere on this site explain the "
        f"same things in plainer language and link back here for the reasoning.{extra}\n"
        f'    <br /><a href="{href}" rel="noopener">View the source file</a>\n'
        "  </div>\n"
        "</div>\n"
    )


def _convert(src: Path, from_kind: str, title_override: str | None = None) -> tuple[str, str]:
    raw = src.read_text(encoding="utf-8")
    m = H1.search(raw)
    if m:
        # Starlight renders the title from frontmatter; leaving the H1 in the
        # body would print it twice.
        body = raw[: m.start()] + raw[m.end() :]
        h1 = m.group(1).strip()
    else:
        body, h1 = raw, src.stem
    title = title_override or h1

    description = _description(body)
    body = _retag_fences(body)
    body = _escape_angles(body)
    body = _rewrite_links(body, from_kind)

    source_rel = src.relative_to(REPO).as_posix()

    front = [
        "---",
        f"title: {yaml_quote(title)}",
    ]
    if description:
        front.append(f"description: {yaml_quote(description)}")
    front += [
        # The reader should edit the ADR, not this file.
        "editUrl: false",
        "# GENERATED FILE — do not edit.",
        f"# Source: {source_rel}",
        "# Regenerate: python website/site.py sync",
        "---",
    ]

    return "\n".join(front) + "\n\n" + _mirror_note(source_rel) + "\n" + body.lstrip("\n"), title


def _index_page(entries: list[tuple[str, str, str]]) -> str:
    """The `/design/` landing page: every decision, one row each, searchable."""
    rows = "\n".join(
        f"| [{num}](adr/{slug}/) | [{name}](adr/{slug}/) |" for num, slug, name in entries
    )
    return f"""---
title: "Design decisions"
description: "Every architectural decision behind Novis, published as written: what was decided, what was rejected, and why."
editUrl: false
tableOfContents: false
# GENERATED FILE — do not edit. Source: docs/adr/README.md
# Regenerate: python website/site.py sync
---

Novis keeps an architecture decision record for every choice that would be
expensive to reverse. Each one states the decision, the alternatives that were
rejected, and the reasoning — including the measurements, where a number decided
it.

They are published here exactly as they were written for the people building the
language. That means they are dense, and they assume you want the argument
rather than the summary. **If you are here to learn how to use Novis, start with
[the documentation](../docs/start/install/) instead** — every guide links back to
the decisions behind it.

Two indexes are worth knowing about:

- **[The ground rules](ground-rules/)** — one sentence per settled decision, with
  a link to the record that owns it. The fastest way to find out whether a rule
  exists.
- **[Divergences](divergences/)** — every place Novis deliberately behaves
  differently from PHP, and the decision that owns each one.

## All decisions

| № | Decision |
|---|---|
{rows}
"""


def main(repo_url: str | None = None, repo_ref: str | None = None) -> int:
    global REPO_URL, REPO_REF
    if repo_url:
        REPO_URL = repo_url.rstrip("/")
    if repo_ref:
        REPO_REF = repo_ref

    if not ADR_DIR.is_dir():
        warn(f"{ADR_DIR} not found — skipping the design mirror")
        return 0

    (OUT_DIR / "adr").mkdir(parents=True, exist_ok=True)

    entries: list[tuple[str, str, str]] = []
    for src in sorted(ADR_DIR.glob("[0-9][0-9][0-9][0-9]-*.md")):
        text, title = _convert(src, "adr")
        (OUT_DIR / "adr" / src.name).write_text(text, encoding="utf-8")
        num = src.name[:4]
        # "ADR 0053 — Iterable/Iterator are …" -> "Iterable/Iterator are …"
        short = re.sub(r"^ADR\s+\d{4}\s*[—–-]\s*", "", title)
        entries.append((num, src.stem, short))

    for leaf, heading in (("ground-rules.md", None), ("divergences.md", None)):
        src = ADR_DIR / leaf
        if src.is_file():
            text, _ = _convert(src, "leaf", title_override=heading)
            (OUT_DIR / leaf).write_text(text, encoding="utf-8")

    (OUT_DIR / "index.md").write_text(_index_page(entries), encoding="utf-8")

    ok(f"design mirror: {len(entries)} decisions + ground rules + divergences")
    return len(entries)


if __name__ == "__main__":
    main()
