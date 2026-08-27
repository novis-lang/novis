"""Scaffold an authored page from the repository sources it will be written from.

This is the seam between the two halves of the content model.

The *facts* live in the repository and are never retyped: this tool copies the
ADR's own `In short` paragraph into the draft as a comment, stamps the source
files with their current hashes, and lays out the page in the shape every
documentation page here uses. The *prose* is then written by a human or an
agent, in language a reader who does not speak English fluently can follow.

Nothing here calls a model. The build stays deterministic and CI needs no API
key; an agent picks the draft up as an ordinary file and fills in the marked
sections, and a human reviews the result as an ordinary diff.

    python site.py draft docs/safety/secret \\
        --title "Secret values" \\
        --from adr/0033-secret-qualifier-for-confidential-values.md

The page shape, which `site.py check` enforces the first two steps of:

    title          what the page is called
    lead           one or two sentences: what it is, what it is for
    quick start    the smallest example that does something real
    ---
    how it works   the detail, with more examples. Technical is fine here.
    ---
    in depth       the full rules, for the reader who came for them
    reference      every form, every option, every edge
"""

from __future__ import annotations

import re
from pathlib import Path

from common import REPO, WEB, blob_hash, fail, ok, warn

DOCS = WEB / "src" / "content" / "docs"


def _resolve(src: str) -> Path:
    p = REPO / src
    if p.exists():
        return p
    if src.startswith("adr/"):
        return REPO / "docs" / src
    return p


def _in_short(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    m = re.search(r"^>\s*\*\*In short:\*\*\s*(.*(?:\n>.*)*)", text, re.MULTILINE)
    if not m:
        return ""
    return re.sub(r"\n>\s?", "\n", m.group(1)).strip()


def main(slug: str, title: str, sources: list[str], force: bool = False) -> int:
    rel = slug.strip("/")
    target = DOCS / (rel + ".mdx")
    if target.exists() and not force:
        fail(f"{target.relative_to(WEB)} already exists (pass --force to overwrite)")
        return 1

    stamped: list[str] = []
    briefs: list[str] = []
    for s in sources:
        p = _resolve(s)
        if not p.is_file():
            fail(f"source not found: {s}")
            return 1
        h = blob_hash(p)
        stamped.append(f"{s}@{h}" if h else s)
        brief = _in_short(p)
        if brief:
            briefs.append(f"### {s}\n\n{brief}")

    if not stamped:
        warn("no --from sources given: this page will have no provenance and no staleness check")

    notes = "\n\n".join(briefs) if briefs else "(no `In short` block found in the sources)"

    body = f"""---
title: {title!r}
description: 'TODO: one sentence, under 160 characters, for search results.'
edit: review
sources:
{chr(10).join('  - ' + s for s in stamped) if stamped else '  []'}
---

import {{ Aside }} from '@astrojs/starlight/components';
import Lead from '~/components/Lead.astro';
import Sources from '~/components/Sources.astro';

{{/*
  WRITING THIS PAGE

  The facts are already decided; they are quoted at the bottom of this comment,
  straight from the sources stamped in the frontmatter above. Your job is the
  language, not the content — and specifically:

    - Short sentences. A reader who learned English second must get the idea
      from the lead alone. If they do not, they leave.
    - No metaphor, no idiom, no jokes that depend on English.
    - Name the thing before you explain it.
    - The first example must do something real and fit on one screen.
    - Do not restate the reasoning behind the decision. Link to the decision.

  Anything you write between {{/* keep */}} and {{/* /keep */}} is never touched
  again by any tool or agent. Use it for a paragraph you have tuned. (In a plain
  `.md` file the same markers are spelled `<!-- keep -->` / `<!-- /keep -->`.)

  ---- from the sources ----

{notes}
*/}}

<Lead>
  TODO: one or two sentences. What is this, and what is it for? No mechanism,
  no milestone, no decision record — those come later on the page.
</Lead>

## Quick start

TODO: the smallest example that does something real. Show it, then say in one
sentence what happened.

## How it works

TODO: the detail. Technical framing is fine from here down — the reader who got
this far wants it. More examples, and cross-links to the pages that own the
neighbouring ideas.

## In depth

TODO: the complete rules. Every form, every option, every edge case, in as much
detail as the feature actually has. Link to the design decision for *why*;
this section is *what*.

<Sources sources={{frontmatter.sources}} />
"""

    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(body, encoding="utf-8")
    ok(f"drafted {target.relative_to(WEB).as_posix()}")
    if stamped:
        ok(f"stamped {len(stamped)} source(s)")
    return 0
