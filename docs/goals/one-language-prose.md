---
looper: 1
title: Each language feature is written once
summary: "Each language feature's about.md is the one text for its chapter section and its syntax page, the syntax pages show its examples, the chapters are generated, and the checks below pass."
context: [website/src/lib/feature.ts, tools/nv/cmd/site.ts, tools/nv/proofs/roster.ts, crates/nvs-cli/src/agent.rs]
stages:
  - id: 1
    title: "the syntax pages show each feature's about.md and examples"
    status: open
    context: [website/src/content/docs/syntax/control-flow/loops.mdx]
  - id: 2
    title: "the chapters under docs/reference/lang are generated from the about.md files"
    status: open
    context: [docs/reference/lang/40-statements.md, crates/nvs-stdlib/build.rs]
checks:
  - id: doc-links
    stage: 2
    name: every doc link and anchor resolves
    run: [bun, nv, verify, --doc]
---

# Each language feature is written once

## Why

Each language feature is written three times by hand. The chapter under `docs/reference/lang/` is
built into the binary for `nvs agent`. The syntax page under `website/src/content/docs/syntax/` is
what a user reads. The feature's `about.md` under `docs/examples/lang/` is a feature proof, and the
website never shows it: `website/src/lib/feature.ts:33` reads every area of `docs/examples/` but
`lang`. The site lock only reports that one copy changed, and the other two are then rewritten by
hand. The `Core` pages already work the way this goal wants: the page shows the member's `about.md`
and its examples.

## Standing decisions

**The user decided** that the three copies become one, after goal `docs-cleanup` lands.

**The agent proposed, and the first session confirms against the code before building:**
- A feature's `about.md` is the one text. A syntax page keeps its own order, headings and links
  between features, and shows each covered feature's `about.md` and examples through the component
  the `Core` pages use.
- A chapter's sections are rendered from the `about.md` files in roster order, so `nvs agent` and
  the hover text read the same words as the website. The chapter's `#` headings stay the roster's
  feature ids.
- Where the syntax page and the `about.md` disagree, the page's text is the newer one, because the
  site's writers checked it against the binary. The session says which it took in the commit.

**Trade-offs.** The website build reads 133 more files, a few seconds. The binary embeds the same
chapters, only generated. A writer edits one file in place of three.

## Stages

### 1. the syntax pages show each feature's about.md and examples

**Does:** every syntax page shows the `about.md` and examples of each feature it covers, and repeats
none of their text.

- [ ] `feature.ts` reads `lang` like the other areas. `website/src/lib/feature.ts:33`
- [ ] Each syntax page renders its covered features, and its own text is cut to what joins them. `website/src/content/docs/syntax/control-flow/loops.mdx:3`
- [ ] The site lock hashes the `about.md` alone for a `lang` feature. `tools/nv/cmd/site.ts:306`

### 2. the chapters under docs/reference/lang are generated from the about.md files

**Does:** no chapter section is hand-written, and `nvs agent` prints the same text the website does.

- [ ] A renderer writes each chapter from its features' `about.md` files, and a `--check` refuses a hand edit. `tools/nv/proofs/roster.ts:28`
- [ ] `nvs agent primer` and `show` read the generated chapters, and their tests pass unchanged in shape. `crates/nvs-cli/src/agent.rs:187`

## Handoff

### State

The goal is new and waits for `docs-cleanup` to land.

### Next

Stage 1, on `website/src/lib/feature.ts` and the syntax pages:
- [ ] `feature.ts` reads `lang`. `website/src/lib/feature.ts:33`
- [ ] One syntax page renders its features, as the pattern for the rest. `website/src/content/docs/syntax/control-flow/loops.mdx:3`

### Backlog
