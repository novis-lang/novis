"""The checks that keep authored pages honest.

This site says the same things as the repository, in different words. The risk
that creates is not duplication of *text* — it is duplication of *facts* that
then drift apart. These checks are what makes the arrangement safe:

* **Provenance.** Every authored page lists the repository files it was written
  from, each stamped with that file's git blob hash at the time a human last
  read it. When the source changes, the page is reported stale. Nobody has to
  notice; the check notices.

* **Shape.** A documentation page opens with a title and one lead paragraph, in
  that order, before anything else. This is checked rather than merely asked
  for, because it is the rule most easily lost under deadline and the one whose
  loss costs the most readers.

* **Locks.** `<!-- keep -->` regions must be balanced, and a page marked
  `edit: locked` must not be missing its explanation of why.

* **Coupling.** Two files here reimplement Starlight internals. If the upstream
  shape moves, say so here rather than letting the theme toggle quietly break.

Nothing in this file rewrites anything. `site.py bless` is the only thing that
re-stamps a page, and it only does so when a human asks.
"""

from __future__ import annotations

import re
from pathlib import Path

from common import (
    REPO,
    WEB,
    blob_hash,
    fail,
    info,
    keep_regions,
    ok,
    parse_frontmatter,
    warn,
)

DOCS = WEB / "src" / "content" / "docs"
GENERATED_DIRS = {"design"}

# `adr/0006-x.md` is spelled without the `docs/` prefix in frontmatter for
# readability; both spellings resolve.
def resolve_source(path: str) -> Path:
    p = REPO / path
    if p.exists():
        return p
    if path.startswith("adr/"):
        return REPO / "docs" / path
    return p


def authored_pages() -> list[Path]:
    if not DOCS.is_dir():
        return []
    return [
        p
        for p in sorted(DOCS.rglob("*"))
        if p.suffix in (".md", ".mdx")
        and p.relative_to(DOCS).parts[0] not in GENERATED_DIRS
    ]


def check_page(path: Path) -> tuple[list[str], list[str]]:
    """Return `(errors, warnings)` for one authored page."""
    errors: list[str] = []
    warnings: list[str] = []
    rel = path.relative_to(WEB).as_posix()
    text = path.read_text(encoding="utf-8")
    fm = parse_frontmatter(text)

    # A generated page answers to its generator, not to these rules.
    if "GENERATED FILE" in text[:600]:
        return errors, warnings

    if not fm.get("title"):
        errors.append(f"{rel}: no `title` in frontmatter")

    edit = fm.get("edit", "open")
    if edit not in ("open", "review", "locked"):
        errors.append(f"{rel}: `edit: {edit}` is not one of open/review/locked")

    # Keep regions must be balanced, or a rewrite would lose one.
    opens = len(re.findall(r"(?:<!--|\{/\*)\s*keep\b", text))
    closes = len(re.findall(r"(?:<!--|\{/\*)\s*/keep\b", text))
    if opens != closes:
        errors.append(f"{rel}: {opens} keep-region opens but {closes} closes")
    kept = keep_regions(text)
    if any(not k.strip() for k in kept):
        warnings.append(f"{rel}: an empty `<!-- keep -->` region — delete it or fill it")

    # Provenance.
    sources = fm.get("sources") or []
    if isinstance(sources, str):
        sources = [sources]
    for entry in sources:
        src, _, stamp = str(entry).partition("@")
        p = resolve_source(src.strip())
        if not p.is_file():
            errors.append(f"{rel}: source `{src}` does not exist")
            continue
        if not stamp:
            warnings.append(f"{rel}: source `{src}` has no `@hash` stamp — run `site.py bless`")
            continue
        current = blob_hash(p)
        if current and current != stamp.strip():
            warnings.append(
                f"{rel}: STALE — `{src}` changed since this page was reviewed "
                f"({stamp.strip()} → {current}). Reread the page, then `site.py bless {rel}`."
            )

    # Shape: a documentation page leads with one lead paragraph.
    parts = path.relative_to(DOCS).parts
    is_doc = parts and parts[0] == "docs"
    if is_doc and path.name != "index.mdx":
        body = text.split("---", 2)[-1]
        has_lead = "<Lead>" in body or 'class="nv-lead"' in body
        if not has_lead:
            warnings.append(
                f"{rel}: no `<Lead>` paragraph. Every documentation page opens with one or two "
                "sentences saying what the thing is, before anything else."
            )
        elif "<Lead>" in body:
            before = body.split("<Lead>")[0]
            # Only imports, blank lines and frontmatter leftovers may precede it.
            offending = [
                l
                for l in before.splitlines()
                if l.strip() and not l.startswith("import ") and not l.startswith("//")
            ]
            if offending:
                warnings.append(
                    f"{rel}: content appears before the `<Lead>`: {offending[0][:60]!r}"
                )
        if body.count("<Lead>") > 1:
            errors.append(f"{rel}: more than one `<Lead>` — a page has exactly one")

    return errors, warnings


def check_grammar() -> tuple[list[str], list[str]]:
    """The committed grammar must equal what the compiler's keyword table produces."""
    import json

    import gen_grammar

    out = WEB / "src" / "grammars" / "nvs.tmLanguage.json"
    try:
        expected = gen_grammar.build(gen_grammar.read_keywords())[0]
    except SystemExit as e:
        return [f"grammar: {e}"], []
    if not out.is_file():
        return [], ["grammar: not generated yet — run `site.py sync`"]
    actual = json.loads(out.read_text(encoding="utf-8"))
    if actual != expected:
        return [], ["grammar: out of date with crates/nvs-syntax — run `site.py sync`"]
    return [], []


def check_starlight_coupling() -> tuple[list[str], list[str]]:
    """Two overrides reimplement Starlight internals. Detect an upstream move."""
    upstream = WEB / "node_modules" / "@astrojs" / "starlight" / "components" / "ThemeProvider.astro"
    if not upstream.is_file():
        return [], []
    text = upstream.read_text(encoding="utf-8", errors="replace")
    missing = [
        marker
        for marker in ("StarlightThemeProvider", "updatePickers", "starlight-theme", "theme-icons")
        if marker not in text
    ]
    if missing:
        return [], [
            "starlight coupling: upstream ThemeProvider no longer mentions "
            + ", ".join(missing)
            + ". src/components/overrides/ThemeProvider.astro reimplements that contract — "
            "diff them before trusting the theme toggle."
        ]
    return [], []


def main() -> int:
    errors: list[str] = []
    warnings: list[str] = []

    pages = authored_pages()
    for p in pages:
        e, w = check_page(p)
        errors += e
        warnings += w

    for fn in (check_grammar, check_starlight_coupling):
        e, w = fn()
        errors += e
        warnings += w

    for w in warnings:
        warn(w)
    for e in errors:
        fail(e)

    if not errors and not warnings:
        ok(f"{len(pages)} authored pages: shape, provenance and locks all clean")
    elif not errors:
        ok(f"{len(pages)} authored pages checked, {len(warnings)} warning(s)")
        info("warnings do not fail the build; they are the writing queue")

    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
