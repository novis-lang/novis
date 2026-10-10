---
looper: 1
title: "Every website page carries search questions, stamped and checked like the page itself"
summary: "When this goal is green, every page of the website has search questions in `website/src/data/questions.json`: four to six questions in the words a visitor types before they know a method's or a setting's name."
context: [tools/nv/cmd/site.ts, website/src/components/Banner.astro]
stages:
  - id: 2
    title: the stamp and the check
    status: open
  - id: 3
    title: the Core reference
    status: open
  - id: 4
    title: configuration and CLI
    status: open
  - id: 5
    title: the handwritten pages
    status: open
  - id: 6
    title: "owed fails, and the second measurement"
    status: open
checks:
  - id: questions-unit
    stage: 2
    name: "site questions: the entry shape, stale, broken and owed, the stamp and the wrap gate"
    run: [bun, test, tools/nv/test/site-questions.test.ts]
    cwd: .
  - id: core-questions
    stage: 3
    name: "site --questions --status: no Core page owes questions"
    run: [bun, nv, site, --questions, --status]
    cwd: .
    stdout_contains: ["questions core: 0 owed, 0 stale"]
  - id: config-cli-questions
    stage: 4
    name: "site --questions --status: no configuration or CLI page owes questions"
    run: [bun, nv, site, --questions, --status]
    cwd: .
    stdout_contains: ["questions config: 0 owed, 0 stale", "questions cli: 0 owed, 0 stale"]
  - id: page-questions
    stage: 5
    name: "site --questions --status: no handwritten page owes questions"
    run: [bun, nv, site, --questions, --status]
    cwd: .
    stdout_contains: ["questions pages: 0 owed, 0 stale"]
  - id: owed-fails
    stage: 6
    name: "site --check questions: no page is owed, stale or broken"
    run: [bun, nv, site, --check, questions]
    cwd: .
extra:
  files: []
  env: {}
  context:
    adrs: []
    milestones: []
    plan: []
    playbook: []
    rules: [testing/website-pages-are-stamped, testing/feature-proofs]
    shapes: [A commit message]
---

# Every website page carries search questions, stamped and checked like the page itself

## Why

### The target

When this goal is green, every page of the website has search questions in
`website/src/data/questions.json`: four to six questions in the words a visitor types before they know
a method's or a setting's name. The site's search (Pagefind) indexes them, so "how do I remove
duplicate values from an array" finds `Core\Arr::unique`. Every entry records the hash of what it was
written from. `bun nv site --check questions` fails on a page whose questions are stale or missing,
and CI runs it, so a feature that changes owes new questions the way it owes a reread of its pages.

### Why a side goal

The user asked for this on 2026-10-09, run by hand and off the chain. Nothing on the chain waits for
it, and its file set is the website and `tools/nv/cmd/site.ts`, which no chain goal opens.

### What is on disk today, measured

- **The questions for `Core\Str` and `Core\Arr`** (95 pages, 425 questions) landed as the proof test:
  `website/src/data/questions.json`, keyed by page URL, values are the questions with no hash yet.
- **`website/src/components/Banner.astro`** writes a page's questions into a
  `data-pagefind-index-attrs="data-questions"` attribute. Pagefind indexes it, the page shows no extra
  text, and Google does not read it as hidden text. The banner is inside Starlight's
  `<main data-pagefind-body>` on every page, the front page included.
- **The proof test, measured on 2026-10-09.** 36 plain-language queries for `Core\Str` and `Core\Arr`
  pages, written before any question existed by a writer the question authors could not read. Right
  page first: 6 without questions, 16 with them. In the top 3: 8 and 26. Not in the top 20: 28 and 9.
  8 control queries outside those classes ranked the same in both builds. The misses were words no
  question used ("batches", "flat", "merge two arrays") and one PHP name ("sprintf").
- **Removing filler words from the query** before Pagefind searches moved two of the nine misses into
  the top 20 and no other. It is not worth a search hook.
- **The hash exists.** `featureHash` (`tools/nv/cmd/site.ts:215`) is a feature's card, its `about.md`
  and its examples. `staleness` (`:261`) and `wrapProblems` (`:295`) are the stale check and the wrap
  gate for handwritten pages, and this goal's check is built the same way.
- **The name list** (`website/src/pages/search/names.json.ts`, `website/src/components/Search.astro`)
  is separate work and already done. It finds a page by its name. The questions find it by what it does.

### Stage 1 — the floor

Main's carried floor, which a side run is always checked against. Never traded.

## Standing decisions

These are the user's calls, made on 2026-10-09, unless marked as mine. No session re-decides one.

- **Questions written by sessions, not an API.** No LLM service is called at build time or by a tool.
- **No function names from other languages** in any question: no `strlen`, `array_unique`, `sprintf`.
- **The site check only.** Questions are not a feature proof. `bun nv proofs` does not list them, and
  `rule:testing/feature-proofs` does not change.
- **No in-browser vector search.** It is out of scope here.
- **A side goal**, run by hand in its own worktree.
- **The questions are text an end user reads.** Pagefind can show them in a result's excerpt, so
  AGENTS.md § *Text an end user reads* holds them: plain English, at most 15 words, no internals.
- **Mine, 2026-10-09:** the questions live in an indexed attribute, never in hidden page text; four to
  six per page; each question belongs to the page that answers it best, and two close pages split the
  phrasing between them (`trim` and `trimStart`, `append` and `appendAll`).
- **Mine, 2026-10-09:** a measurement is a blind query set run through `pagefind.js` in a browser
  against two builds, and the rank of the right page is what is compared.
- **ADR slots: none.** Decision records are retired. What outlives the goal goes into the module doc
  of `tools/nv/cmd/site.ts` and into `rule:testing/website-pages-are-stamped`, which this goal extends to
  say a page's questions are stamped as the page is.
- **The tradeoffs, stated once.** Visitors get a search that finds a page by what it does. The proof
  test's 95 pages grew `dist/pagefind` from 5.0 MB to 5.1 MB, and Pagefind still loads only the shards
  a query needs. The language and the runtime do
  not change. Contributors gain one more thing a changed feature owes: rewriting and stamping its
  questions, which the site check names.

## Stages

### 2. the stamp and the check

**Does:** Records in every entry of `questions.json` the feature it was written from and that feature's hash, and adds the `questions` part of `bun nv site --check` with the tool a session uses to write questions.

File set: `tools/nv/cmd/site.ts`, `tools/nv/test/site-questions.test.ts` (new),
`website/src/data/questions.json`, `website/src/components/Banner.astro`.

- **The entry's shape** is `{ "feature": <id>, "hash": <hash>, "questions": [...] }`, keyed by page URL
  without the site's base. A reference page's feature is its proofs roster id and its hash is
  `featureHash`. A Core class page and a configuration block page hash their own section. A handwritten
  page's feature is its key under `website/src/content/docs/` and its hash is the file's content.
- **`--check questions`** reports each entry whose hash moved (**stale**), each URL no page has
  (**broken**), and each built page with no entry (**owed**). Stale and broken fail from this stage on.
  Owed fails from stage 6 on.
- **`--questions --status`** prints one line per area, `questions <area>: <n> owed, <n> stale`, and
  exits 0. The areas are `core`, `config`, `cli` and `pages`. The stage checks below read it.
- **`--questions --owed [<area>] [--limit <n>]`** prints each owed or stale page with what its questions
  are written from: URL, name, signature, card and `about.md` for a reference page, the page text for a
  handwritten one. This replaces the extraction script the proof test used.
- **`--questions --stamp <url>...`** records the current feature and hash for each named entry. It is
  run by the session that wrote or reread the questions, never to quiet the check.
- **The wrap gate** refuses stale questions the session made stale itself, as `wrapProblems` does for
  pages. Add it beside that function, not as a second gate.
- **Stamp the 95 entries already on disk**, and switch `Banner.astro` to the new shape.

### 3. the Core reference

**Does:** Writes the questions for every Core class and method page.

File set: `website/src/data/questions.json`.

Take the classes in alphabetical order, as many per session as fit. Split a session's classes over
subagents, one class group each, as the proof test did. Each subagent reads only the `--owed` output
for its classes.

### 4. configuration and CLI

**Does:** Writes the questions for every configuration block and key, and every CLI page.

File set: `website/src/data/questions.json`.

A configuration key's questions go on its block's page, because the key has no page of its own.

### 5. the handwritten pages

**Does:** Writes the questions for every page under Guides, Syntax, In depth and Install, and the front page.

File set: `website/src/data/questions.json`.

### 6. owed fails, and the second measurement

**Does:** Makes an owed page fail `--check questions`, and measures the search again on a new blind query set across every area.

File set: `tools/nv/cmd/site.ts`, `tools/nv/test/site-questions.test.ts`.

- From this stage on, a new feature's page fails CI until it has questions.
- The measurement repeats the proof test's method: a writer who cannot see the questions writes about
  40 queries across all four areas, with the right page for each, and the session compares a build
  without `questions.json` to one with it. The numbers go into this goal's handoff for the user.

## Handoff

### State

**Side goal `search-questions`: the proof test landed, the stamp has not.** `website/src/data/questions.json` has 425 questions for the 95 `Core\Str` and `Core\Arr` pages, keyed by URL, with no feature and no hash. `website/src/components/Banner.astro` writes them into an attribute Pagefind indexes. Nothing in `tools/nv` reads the file yet. The goal's prose has the proof test's numbers and the method.

### Next

**Stage 2: the stamp and the check** one file set: `tools/nv/cmd/site.ts`, `tools/nv/test/site-questions.test.ts` (new), `website/src/data/questions.json`, `website/src/components/Banner.astro`.

- [ ] **The entry shape and the hash**: `{ feature, hash, questions }` per URL, the hash from `featureHash` for a reference page and from the file for a handwritten one. Anchor: `tools/nv/cmd/site.ts:215`.
- [ ] **`--check questions`, `--questions --status`, `--owed` and `--stamp`**, built like `staleness`, with the unit test. Anchor: `tools/nv/cmd/site.ts:261`.
- [ ] **The wrap gate** for stale questions the session made itself, beside the one for pages. Anchor: `tools/nv/cmd/site.ts:295`.
- [ ] **Stamp the 95 entries on disk** and switch the banner to the new shape. Anchor: `website/src/components/Banner.astro:1`.

### Backlog

- Then stages 3 to 6 in order (docs/agent/goals/side/search-questions.md).
- Then the goal's end gates: `bun nv verify --doc` and `bun nv owners --closes search-questions` (docs/agent/goals/side/search-questions.md).
