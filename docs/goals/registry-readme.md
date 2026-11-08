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

- [ ] `bun nv affected --run`, every failure fixed in one pass, then once more.
- [ ] The goal-end gates of AGENTS.md § *In the loop*.

## Handoff

### State

Stage 1 is done. `apps/registry/src/Readme.nvs` renders the readme once in `Publish::record`, into
the new column `releases.readme_html`, and `PackagePage::readme` shows it through `Readme::show`
(standing decisions 4 to 6). The rule
`docs/rules/packaging/search-sorts-by-the-readers-choice.md` says how the readme is shown and
declines the two other ways, and the gap is deleted.

The sweep's reds were stale again: no `env-secret-proofs` check is defined, and nightly run
42359979484 ran on 26dda36627, before the Miri ignores in f13f40646a.

Run from `apps/registry` with `nvs test src --filter PublishTest`, the publish tests fail with "the
program wrote no body" on the tree without this change too. Their real runner is `bun nv verify`'s
app suite, so judge them there.

### Next

Stage 2: `bun nv affected --run`, then the goal-end gates. Expect no test red beyond the app suite's
`PackagePageTest` and `PublishTest` (`apps/registry/src/`), which this change touches.

### Backlog

Nothing yet.
