---
looper: 1
title: The registry's package page shows each version's readme rendered from Markdown
summary: "Reached when each package version's readme is rendered once by Core\\Markdown::toHtml when the registry builds it, stored with the version, and shown on the package page, with relative links and images pointing at the commit it was built from and heading anchors that work, and no page view parses Markdown."
context: [apps/registry/src/PackagePage.nvs, crates/nvs-cli/src/package/build.rs, docs/rules/packaging/search-sorts-by-the-readers-choice.md]
stages:
  - id: 1
    title: "the readme is rendered once per version, stored, and shown on the package page"
    status: open
    context: [apps/registry/src/Builder.nvs, docs/rules/core-classes/markdown-to-html.md]
  - id: 2
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks: []
---

# The registry's package page shows each version's readme rendered from Markdown

## Why

A package page shows its readme as plain text. `rule:packaging/search-sorts-by-the-readers-choice`
declined a rendered readme because `Core` had no Markdown renderer, and one for text any publisher
writes is a new way to inject markup. The `markdown` goal ships `Core\Markdown`, which removes that
half. What is left is the page's work, and this goal does it.

## Standing decisions

1. **Rendered once, when the version is built.** `toHtml` runs where the registry stores the readme,
   and the `Markup` is stored beside the text. A page view reads the stored HTML and parses nothing.
   It spends one stored copy of the rendered readme per version, bounded by the readme's own cap.
2. **Links point at the build's commit.** A relative link or image is rewritten through the `link`
   and `image` callbacks to the repository's file at the commit the version was built from. A link
   to another host keeps its URL.
3. **Heading anchors** come from the `heading` callback, as a slug of the heading's text.

## Stages

### 1. the readme is rendered once per version, stored, and shown on the package page

- [ ] The render at build time, with the three callbacks. `apps/registry/src/Builder.nvs:1`
- [ ] The package page shows the stored HTML. `apps/registry/src/PackagePage.nvs:1`
- [ ] The Declined entry is removed, and the rule says how the readme is shown.
  `docs/rules/packaging/search-sorts-by-the-readers-choice.md:61`
- [ ] The gap `data/gaps/registry/readme-is-shown-as-text.json` is deleted.

### 2. the goal is verified once, and every failure is fixed

- [ ] `bun nv affected --run`, every failure fixed in one pass, then once more.
- [ ] The goal-end gates of AGENTS.md § *In the loop*.

## Handoff

### State

Written by the `markdown` goal's stage 2, which owes this page's readme a renderer. Nothing is built
yet, and it waits for `markdown` to ship `Core\Markdown::toHtml`.

### Next

Stage 1, from its first item, once `markdown` is reached.

### Backlog

Nothing yet.
