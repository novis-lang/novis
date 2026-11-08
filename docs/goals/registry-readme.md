---
looper: 1
title: The registry's package page shows each version's readme rendered from Markdown
summary: "Reached when each package version's readme is rendered once by Core\\Markdown::toHtml when the registry builds it, stored with the version, and shown on the package page, with relative links and images pointing at the commit it was built from and heading anchors that work, and no page view parses Markdown."
context: [apps/registry/src/PackagePage.nvs, crates/nvs-cli/src/package/build.rs, docs/rules/packaging/search-sorts-by-the-readers-choice.md]
stages:
  - id: 1
    title: "the readme is rendered once per version, stored, and shown on the package page"
    status: done
    context: [apps/registry/src/Builder.nvs, docs/rules/core-classes/markdown-to-html.md]
  - id: 2
    title: "the goal is verified once, and every failure is fixed"
    status: done
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
4. **A page view rebuilds the stored HTML through `Core\Html::sanitize`.** `Markup` has no other
   constructor from a stored string, and the rebuild keeps a row changed outside the registry from
   writing a script (security over latency). It costs one HTML parse per view, and no Markdown parse.
5. **The file addresses use `/blob/<commit>/` and `/raw/<commit>/`**, which GitHub and GitLab serve.
   A trailing `.git` on the repository is dropped. A path that leaves the repository gives `null`, so
   the link writes its text alone. A `#slug` link becomes `#md-slug`.
6. **A database made before `readme_html` renders each stored readme once**, when `Schema` adds the
   column (`Schema::renderReadmes`).

## Stages

### 1. the readme is rendered once per version, stored, and shown on the package page

- [x] The render at build time, with the three callbacks. `apps/registry/src/Builder.nvs:1`
  - [x] `Readme::render` with `link`, `image` and `heading`. `apps/registry/src/Readme.nvs`
  - [x] `Publish::record` stores `readme_html`, and `Schema` adds and fills the column.
- [x] The package page shows the stored HTML. `apps/registry/src/PackagePage.nvs:1`
  - [x] `PackagePage::readme` shows `Readme::show` of `readme_html`.
  - [x] `PackagePageTest::theReadmeIsShownAsHtml` replaces `theReadmeIsEscaped`, and passes.
- [x] The Declined entry is removed, and the rule says how the readme is shown.
  `docs/rules/packaging/search-sorts-by-the-readers-choice.md:61`
- [x] The gap `data/gaps/registry/readme-is-shown-as-text.json` is deleted.

### 2. the goal is verified once, and every failure is fixed

- [x] `bun nv affected --run`, every failure fixed in one pass, then once more.
- [x] The goal-end gates of AGENTS.md § *In the loop*.

## Handoff

### State

The goal is reached. `apps/registry/src/Readme.nvs` renders the readme once in `Publish::record`,
into `releases.readme_html`, and `PackagePage::readme` shows it through `Readme::show`. Every
standing decision is stated in `Readme.nvs`'s doc comment or in
`docs/rules/packaging/search-sorts-by-the-readers-choice.md`.

Stage 2's gate failed twice, and both are fixed: `docs/rules/packaging.md` was not rendered again
after stage 1 changed its fragment, and `bun nv links` read the `https://` inside a Markdown string
in `PackagePageTest.nvs` as a comment (`tools/nv/cmd/links.ts`, `TRAILING_COMMENT_RE`). Then
`bun nv affected --run`, `bun nv verify --doc`, `bun nv owners --closes registry-readme` and
`bun nv ci --check` were all green.

The sweep's reds were stale: no `env-secret-proofs` check is defined, and nightly run 42359979484
ran on 26dda36627, before the Miri ignores in f13f40646a.

### Next

Retire the goal. No test is expected red.

### Backlog

Nothing yet.
