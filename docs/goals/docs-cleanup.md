---
looper: 1
title: "docs/ holds only what the project's own work needs, and says each thing once"
summary: "docs/plan, docs/spec, docs/perf, docs/design.md, docs/future-ideas.md and docs/implementation-plan.md are gone with the tooling that read them, every public page is on the website alone, AGENTS.md is one document, docs/agent says each thing once, each language feature's about.md is the one text for its syntax page and its chapter section, and the checks below pass."
context: [AGENTS.md, docs/agent/doc-cleanup.md, website/README.md, tools/nv/cmd/rules.ts]
stages:
  - id: 1
    title: the plan and the design files are gone
    status: open
    context: [tools/nv/cmd/rules.ts, docs/design.md]
  - id: 2
    title: "the Core spec is gone, and the website's Core pages are built from the registry"
    status: done
    context: [docs/spec/01-core-library.md, tools/nv/renderers/website-core.ts, tools/nv/renderers/spec-tables.ts, crates/nvs-stdlib/tests/spec_registry_coverage.rs]
  - id: 3
    title: "docs/perf is gone, its ledgers are data and its findings have homes"
    status: done
    context: [tools/nv/proofs/ledger.ts, tools/nv/proofs/perf.ts, tools/nv/cmd/scaling.ts, docs/goals/fewer-allocations.md]
  - id: 4
    title: every public page is on the website alone
    status: done
    context: [docs/release.md, tools/nv/cmd/release.ts, tools/nv/cmd/reference.ts, website/src/content/docs/install/index.mdx]
  - id: 5
    title: "AGENTS.md is one document, and docs/agent says each thing once"
    status: done
    context: [docs/agent/commands.md, docs/agent/conventions.md, docs/agent/playbook.md, looper.toml]
  - id: 6
    title: "each language feature's facts are in its about.md, its details.md and its examples, once"
    status: open
    context: [docs/reference/lang/40-statements.md, docs/examples/lang/statements/break-and-continue/about.md, website/src/content/docs/syntax/control-flow/break-continue-return.mdx, tools/nv/proofs/roster.ts]
  - id: 7
    title: "the syntax pages show each feature's about.md, details and examples"
    status: open
    context: [website/src/lib/feature.ts, tools/nv/cmd/site.ts, website/src/content/docs/syntax/control-flow/break-continue-return.mdx]
  - id: 8
    title: the chapters under docs/reference/lang are generated from about.md and details.md
    status: open
    context: [tools/nv/proofs/roster.ts, crates/nvs-cli/src/agent.rs, tools/nv/cmd/reference.ts]
checks:
  - id: rules
    stage: 1
    name: "the rulebook is clean, and no file cites a deleted doc"
    run: [bun, nv, rules, --check]
  - id: doc-links
    stage: 4
    name: every doc link and anchor resolves
    run: [bun, nv, verify, --doc]
  - id: lint
    stage: 5
    name: "goals, chain, README and playbook lint clean"
    run: [looper, lint]
  - id: playbook-retired
    stage: 5
    name: no playbook bullet is past its until condition
    run: [looper, playbook, retire, --check]
---

# docs/ holds only what the project's own work needs, and says each thing once

## Why

The website is now the home of everything public: the guides, the syntax, the `Core` reference, the
configuration and CLI pages. Much of `docs/` was written before it existed. A pass on 2026-10-17
read every file under `docs/agent`, `docs/perf`, `docs/plan`, `docs/spec` and the `docs/` root
against the website, the rules, the goals and the code. Most of it repeats a rule, a module doc or
a website page, describes a tool that is gone, or records a finished pass. Some of it is still read
by tools, so each deletion takes its tooling with it.

## Standing decisions

**The user decided:**
- **Novis works without looper** (AGENTS.md § *Novis works without looper*). Nothing is moved,
  replaced or deleted so that the work needs looper. Stage 5's trim of AGENTS.md keeps every `bun nv`
  command a session without looper uses, such as `bun nv affected --run` for verify, even where the
  managed block names a `looper` command.
- **The website wins for anything public.** `docs/` keeps only what developers of Novis need for
  the project's own work. Where a doc repeats the website, the doc's copy is deleted. Where a doc has
  public content the website lacks, it moves to the website first, written to AGENTS.md § *Text an
  end user reads*.
- **`docs/plan/`, `docs/spec/`, `docs/perf/`, `docs/design.md`, `docs/future-ideas.md` and
  `docs/implementation-plan.md` are deleted.** There are zero milestones, so the milestone
  machinery goes with the plan. This overrides `lean-loop`'s earlier "the milestone files stay".
- **The Core spec is deleted fully in this goal.** The website builds every `Core` page from the
  registry (`nvs meta --json`), the class intros under `docs/reference/core/` and each member's
  `about.md`. Every citation of the spec is rewritten.
- **`docs/novis.md` becomes the website's `/llms-full.txt`**, made by the same generator at build
  time. Nothing is committed under `docs/` for it. The proof that every chapter example runs stays.
- **`docs/reference/` stays as the source it is** (the binary, the roster, the lock). Only its
  public-only pieces move to the website.
- **Each fact about a language feature is written once** (stages 6 to 8). Today each feature is
  written three times by hand, at three depths: the chapter under `docs/reference/lang/` (dense,
  every edge case, built into the binary for `nvs agent`), the syntax page (a plain manual with its
  own snippets), and the feature's `about.md` with its examples, which the website never shows
  (`website/src/lib/feature.ts:33` reads every area of `docs/examples/` but `lang`). The site lock
  only reports that one copy changed. The user chose the full version:
  - **`about.md`** is the plain explanation, up to 200 words, as for every feature.
  - **A new `details.md`** beside it holds the exact rules and edge cases the chapter has and
    `about.md` does not. Each fact is in exactly one of the two.
  - **The examples under `docs/examples/lang/`** are the only code. A chapter code block that shows
    something no example shows becomes an example, and the website snippets for these features are
    deleted.
  - **A syntax page** keeps its title, its order, the links between features, *Use this when* and
    *Do and don't*. For each feature it covers, it shows the `about.md`, the examples and the
    details, through the component the `Core` pages use.
  - **A chapter is generated** from its features' `about.md` and `details.md` in roster order, so
    `nvs agent`, the hover text and `/llms-full.txt` read the same words as the website. The
    chapter's `#` headings stay the roster's feature ids.
- **The rules `testing/perf-history-file` and `testing/perf-secondary-figures` are retired**, with
  `docs/perf/history.ndjson`. Nothing writes that file, and its PHP ratio conflicts with measuring
  Novis against itself only.
- **This goal takes over `lean-loop`'s stage 3 whole and its plan item from stage 4.** `lean-loop`
  keeps its tool deletions.

**Carried from `lean-loop`, decided there by the agent and kept:**
- AGENTS.md takes looper's own shape. The project's text outside the managed markers holds only what
  the managed block does not say: the priority ordering, text an end user reads, neutral names, the
  cargo `-p` rule, feature proofs and where Novis departs from the stack files. Where the two halves
  contradict, the managed block wins unless a Novis rule decided otherwise, and that Novis rule moves
  into a rule fragment or `docs/agent/` and is named from the block's table.
- The playbook keeps a bullet only while its trap can still happen. A bullet about a removed tool, a
  bullet that tells a story, and a bullet that contradicts a rule are deleted, never reworded.
  `[orient] traps` drops from 20 to 8.

**The agent's calls for stages 6 to 8:**
- `details.md` is held to AGENTS.md § *Text an end user reads*, like `about.md`, because the website
  shows it. It has no word limit. A feature whose chapter section adds nothing has no `details.md`.
- Where two texts disagree on a fact, the binary decides: the session runs the case and writes what
  it does. The commit names each disagreement it found.
- An example that shows a compile error needs the examples tree to accept one. Whether it does is
  not checked; if it does not, stage 6 adds that first.
- Stage 6 is content and runs one chapter at a time, before the website and the generator change,
  so every slice leaves all three texts consistent.

**The method is [doc-cleanup.md](../agent/doc-cleanup.md).** A rule id is never renamed. A unique
fact that is still true moves to its home before its file is deleted: a rule fragment, a module's
`//!` doc, a gap record under `data/gaps/`, a goal, or a website page. A historical fact, a finished
pass and a measured figure the code does not enforce are dropped. `git log` is the history.

**No product behaviour changes,** except the website: its `Core` pages gain the summaries and
constants the spec hid, it gains `/llms-full.txt` and the pages stage 4 moves, and its syntax pages
show each feature's examples.

**Trade-offs.**
- Every session reads fewer tokens, and no tool reads a file that only repeats another.
- The website build gains one step for `/llms-full.txt` and reads the 133 language features' files,
  a few seconds, with no runtime cost. The binary embeds the same chapters, only generated.
- A syntax page gets longer: about three examples per feature, where most show one snippet today,
  and the details under each feature. `nvs agent` reads in the plain style, with nothing lost.
- Stage 6 is the largest stage of the goal, roughly 15 to 20 sessions. That figure is a guess from
  the size of the chapters (7,617 lines), not a measurement.
- After it, a change to a language feature is one edit in one place, and no copy can drift.
- The spec's coverage test goes, because the registry becomes the only source for a signature.
  Nothing in the spec designed a member the registry lacks: every "outstanding" ratchet is empty.

## Stages

### 1. the plan and the design files are gone

**Does:** no plan index, milestone, status block or frozen design remains, and what only they said
is in its home.

- [x] The unique facts of `docs/plan/design.md` move to their homes: the premise table to `benches/abi-probe/src/lib.rs`'s module doc; the execution tiers, the safepoint list and the SIMD stance to `crates/nvs-codegen/src/lib.rs`; the value tags to `crates/nvs-runtime/src/value.rs`; the unsafe policy paragraphs to `tools/nv/cmd/lints.ts`'s header. The two rows that are false today are dropped. `docs/plan/design.md:1`, `benches/abi-probe/src/lib.rs:4`, `crates/nvs-runtime/src/value.rs:3`, `tools/nv/cmd/lints.ts:20`
- [x] The plan's Toolchain field moves into `docs/setup.md`, and `dependency-update.md` names that as its home. `docs/implementation-plan.md:1`, `docs/setup.md:33`, `docs/agent/dependency-update.md:48`
- [ ] `docs/plan/`, `docs/implementation-plan.md`, `docs/design.md`, `docs/future-ideas.md` and `data/plan/status.json` are deleted. `nv plan` and its tests, `schema/milestone.ts` and its foreign keys, `owners.ts`'s milestone kind, `pack.ts`'s milestones, `wrap.ts`'s `PLAN_KINDS`, CI's `plan --check` and the settings entry go with them. `tools/nv/cmd/plan.ts:200`, `tools/nv/schema/milestone.ts:1`, `tools/nv/schema/gap.ts:18`, `tools/nv/schema/rule.ts:22`, `tools/nv/lib/wrap.ts:19`, `.github/workflows/ci.yml:978`, `.claude/settings.json:60`
  - [x] the tooling: `nv plan`, `wrap.ts`, both schemas, `data/plan/`, the milestone kind and fields, CI and settings
  - [x] `docs/plan/`, `docs/implementation-plan.md`, `docs/future-ideas.md` and their rows in CONTRIBUTING.md's layout listing
  - [ ] `docs/design.md`, once looper's managed block in AGENTS.md stops linking it: today it is a five-line pointer to the rules and the module docs, because `looper lint` refuses a hand edit inside the markers
- [x] Every citation of those files is rewritten to the new home or removed, including the ones already wrong today (`Cargo.toml`'s "Unsafe policy", `crates/nvs-host/src/scheduler.rs:11`). `rules.ts`'s `RECORD_CITE` refuses a new citation of a deleted doc. `tools/nv/cmd/rules.ts:83`
  - [x] the crates, the manifests, `docs/agent/`, `docs/perf/`, `looper.toml`, the gap record and `first-packages.md`'s context repointed
  - [x] `rules.ts`'s `deletedDocFindings` refuses a citation by path or relative link outside `docs/goals/`, with cases in `rules.test.ts`
- [x] AGENTS.md, CONTRIBUTING.md, `commands.md`, `conventions.md` and `doc-style.md` lose the plan step, the status block and the milestone text. `AGENTS.md:336`, `CONTRIBUTING.md:128`, `docs/agent/conventions.md:535`, `docs/agent/doc-style.md:75`

### 2. the Core spec is gone, and the website's Core pages are built from the registry

**Does:** every signature, summary, enum and constant on the website comes from the registry, the
class intros and the `about.md` files, and no file cites the spec.

- [x] `website-core.ts` builds every class from the registry: the summary is the registry's short doc plus the class intro under `docs/reference/core/`, and every registered constant and enum is published. `tools/nv/renderers/website-core.ts:659`, `tools/nv/renderers/website-core.ts:758`
  - [x] `registryClasses` builds classes, members, shape types, constants and owned enums from `nvs meta --json`
  - [x] the summary is the card's sentence plus the introduction, and `blocksHtml` renders its fences and tables
  - [x] the signature linker reads `Core\Arr::Order` and an enum case `Core\Hash::Digest::Sha256` as one name, and the site builds
- [x] The spec's machinery is deleted: `parseSpec`, the `spec-tables` renderer, the `spec_core_members` record and schema, `website/config/spec-overrides.mjs`, the "Spec note" and unparsed-row UI, the spec tests in `render.test.ts`, the spec path in `php-names.test.ts`, the existence check in `gaps.ts`, and `spec_registry_coverage.rs` with its four empty ratchets. `tools/nv/renderers/spec-tables.ts:17`, `tools/nv/cmd/gaps.ts:509`, `crates/nvs-stdlib/tests/spec_registry_coverage.rs:725`
  - [x] the renderer, the record, its schema, the overrides, the UI, the tests in `render.test.ts` and the check in `gaps.ts`
  - [x] `spec_registry_coverage.rs`, its four ratchets, `owners.ts`'s ratchet register, the rules' `guardedBy` and the six playbook bullets about them
  - [x] the spec path in `php-names.test.ts`, which goes with `docs/spec/` itself
- [x] The spec's unique conventions become rules: a variadic member takes no options shape (`rule:core-api/options-bag`), a `readonly` property of a `Core` value object is read as `->x()`, and what is a member and what is an operator (`rule:core-api/one-paradigm-per-operation`). `00-overview.md`'s unique facts go to `rule:types/grammar`, the lexer's and the parser's module docs and `rule:security/isolate-shares-nothing`. The fragments that call the spec authoritative are reworded. `docs/rules/core-api/field-wise-precedence.md:1`, `docs/rules/core-api/tier-roster.md:37`, `crates/nvs-syntax/src/lexer.rs:600`
  - [x] the preamble's conventions: options-bag, one-paradigm-per-operation, a-lifetime-is-an-object; field-wise-precedence, parameters-are-callable-by-name, reference-card and tier-roster reworded
  - [x] `00-overview.md`'s unique facts: reserved-spellings-are-lower-case, alias-is-never-a-bare-class, `80-concurrency.md`
  - [x] the sections' unique facts: types/arrays, members-are-full-words, throwable-hierarchy, and the module docs of `str`, `arr`, `math`, `cldr`, `time`, `regex`, `encoding`, `hash`, `objmap`, `validate`
- [x] A script rewrites every citation of the spec, about 630 files: `spec § N` becomes the class it names, or is dropped where a `rule:` id stands beside it. `docs/spec/` is deleted and joins `RECORD_CITE`. `tests/conformance/core/`, `crates/nvs-stdlib/src/arr.rs:1`
  - [x] `crates/`, `Cargo.toml`, `docs/agent/`, `docs/rules/`, `findings.md`, `examples/`, and the cards and diagnostics that named the spec
  - [x] `tests/conformance/`, about 515 files
  - [x] `docs/spec/` deleted with `php-names.test.ts`'s path, `ci-changes.ts`'s lane and the primer playbook bullet; it joins `DELETED_DOC_CITE`
  - [x] bare `§ N` citations that meant the spec, which the `spec §` grep misses, under `crates/` and `tests/conformance/`
- [x] `website/README.md` says how the Core reference is built now. `website/README.md:43`

### 3. docs/perf is gone, its ledgers are data and its findings have homes

**Does:** the perf ledgers live under `data/perf/`, and every open finding in the perf prose is a gap
record, a module doc or a line in its goal.

- [x] `members.ndjson`, `members.md` and `bench-budget.json` move to `data/perf/`, with every path constant, the `ci-changes.ts` lane, `brief.ts`'s homes and the rule records' `guardedBy`. `tools/nv/proofs/ledger.ts:11`, `tools/nv/proofs/perf.ts:36`, `tools/nv/cmd/scaling.ts:156`, `tools/nv/cmd/ci-changes.ts:91`, `data/rules/testing/member-perf-ledger.json:10`
- [x] `history.ndjson` is deleted, and the rules `testing/perf-history-file` and `testing/perf-secondary-figures` are retired.
- [x] The three open findings with no record become gap records: an unread POST body of 16 KB or more can lose the response, a head of 100 header lines is answered 431, and `nvs lsp` grows near-quadratically. `docs/perf/performance-pass.md:1`
- [x] `performance-review.md`'s *Bounded* costs go into each module's `//!` cost section, and `scaling --areas --reviewed` is retired with it. `tools/nv/cmd/scaling.ts:1046`
- [x] `userland-gap.md` and `brainsort-evaluation.md` give their still-true lines to `crates/nvs-runtime/src/callable.rs`, `crates/nvs-stdlib/src/sort.rs` and `crates/nvs-stdlib/src/arr.rs`. Goal `fewer-allocations` states its own design where it cites § F, § I and the `Markup` decision, and its stage 9 loses the items this stage does. `docs/goals/fewer-allocations.md:91`, `docs/goals/fewer-allocations.md:167`
  - [x] `sort.rs` says where the key sort is slower and what stays on the merge sort; `nvs-codegen`'s module doc says the tier is not where time goes
  - [x] `fewer-allocations` states the `Markup`, hash and format designs inline
- [x] `performance-pass.md`'s still-true costs go to their module docs or rules, and the file is deleted. `docs/perf/performance-pass.md:1`
- [x] Every citation of `docs/perf/` is rewritten, including `perf_guards.rs`'s assertion message, `userland.ndjson` moves to `data/perf/`, and `docs/perf/` joins `DELETED_DOC_CITE`. `benches/abi-probe/tests/perf_guards.rs:2814`

### 4. every public page is on the website alone

**Does:** a user finds every public fact on the website, and `docs/` holds no copy of one.

- [x] `docs/docker.md`'s public parts join `guides/production/docker.mdx`: the image and its tags, the variants, Compose, CLI use, stopping, health probes and verifying. The stale "one process on one core" section is dropped. Building locally and *Why not Alpine* go to `docker/README.md`. `release.yml`'s documentation label, the Dockerfile, `smoke.sh` and `.gitignore` are repointed. `.github/workflows/release.yml:588`, `docker/Dockerfile:15`
- [x] `docs/why-tainted-and-secret.md`'s three readers, its comparison table and its five points become a website page under `in-depth/`. *What not to claim yet* goes to `rule:programs/three-claims`. `README.md:17` is repointed. `website/src/content/docs/in-depth/concepts/tainted-input.mdx:1`
- [x] `docs/release.md` keeps the procedure. *Verifying a published binary* goes to the install page, the user-facing half of the version scheme goes to the website, and its wrong counts are fixed. `docs/release.md:180`, `tools/nv/cmd/release.ts:88`
- [x] `bun nv reference` writes `/llms-full.txt` into the website build, and `docs/novis.md`, its CI `--check` and verify's staleness step are deleted. `tools/nv/cmd/reference.ts:786`, `.github/workflows/ci.yml:892`, `tools/nv/cmd/verify.ts:1047`
  - [x] `nv render --website`'s `website-llms` renderer writes `website/public/llms-full.txt` from `reference.ts`'s `build`, reading the binary `website-core` reads
  - [x] `nv reference` is the example run alone, in verify and in CI's `reference` job, whose `render --website --check` now covers the file
  - [x] `docs/novis.md` is deleted, its citations repointed, and its path joins `DELETED_DOC_CITE`
- [x] `docs/reference/tools/30-php-differences.md` gets a website page, and `README.md:44` is repointed. `docs/reference/findings.md`'s open items become gap records, and the file is deleted with its citations. `tests/conformance/core/arr-sort-orders-decimals.nvst:9`
  - [x] the website's *Coming from PHP* page shows the chapter's three features through `FeatureSection.astro`, and the README links it
  - [x] `findings.md`'s open items (U13, M10 and any the triage still counts) become gap records or rule text
  - [x] the eight files that cite `findings.md` are rewritten, the file is deleted, and its path joins `DELETED_DOC_CITE`

### 5. AGENTS.md is one document, and docs/agent says each thing once

**Does:** AGENTS.md says each thing once, every file under `docs/agent/` is current and unique, and
the playbook holds only traps that can still happen.

- [x] AGENTS.md outside the markers holds only what the managed block lacks, and no rule number, step count or budget contradicts it. The text names the mid-work check, `bun nv verify --fast -p <crate>` after each slice, and the shell shapes the guard refuses, each with the call to make instead. `AGENTS.md:1`, `AGENTS.md:392`
- [x] `commands.md` and `conventions.md` lose every section another home holds, every measured figure and every story, and the dead `§ Names` anchor and section references are fixed. `docs/agent/commands.md:24`, `docs/agent/conventions.md:494`, `docs/agent/testing.md:16`
  - [x] `commands.md`'s figures and stories go, and its *Disk* and WSL sections point at `disk.ts`, `mirror.ts`, `Cargo.toml` and `tests_without_pdb.rs`
  - [x] `conventions.md` loses its stories, the plan, the spec's test and the measured link count
  - [x] `conventions.md`'s pack heading is fixed; `testing.md` is looper's managed text, so its dead `§ Names` link is looper's template's to fix
- [x] `optimization-prompt.md`, `carried-refusals.md` (with its readers in `holes.ts`, `owners.ts` and `refusals.rs`) and `guard-name-debt.md` are deleted; its three causes move to `loop-authoring.md` § 3, beside a check's `tests` list. `coordinator.md` and `loop-authoring.md` lose their history and stale numbers. `docs/agent/coordinator.md:19`, `tools/nv/cmd/owners.ts:38`
- [x] The playbook loses every bullet about a removed tool, every story, every bullet that contradicts a rule (such as the `git stash` one) and every duplicate, and `[orient] traps = 8`. `docs/agent/playbook/running-things/a-before-after-measurement-is-worth-a-git-stash-and-the-base.md:1`, `looper.toml:8`
  - [x] the two bullets `looper playbook retire --check` named are retired
  - [x] no bullet names a removed `bun nv` command; the carried-refusals bullet goes with that file's item
  - [x] the `git stash` bullet is deleted
  - [x] the stories and the duplicates are deleted, one playbook directory per slice
  - [x] `[orient] traps = 8` in `looper.toml`
- [x] `how-novis-is-written.mdx`'s count of notes comes from `numbers.json` or goes. `website/src/content/docs/in-depth/how-novis-is-written.mdx:62`

### 6. each language feature's facts are in its about.md, its details.md and its examples, once

**Does:** for every language feature, every fact its chapter section, its syntax page and its
`about.md` state is in exactly one of `about.md`, `details.md` or an example, and the binary agrees
with each.

- [x] The proofs tooling knows `details.md`: optional, under the feature's examples folder, and held to the comment bounds. Whether an example can show a compile error is checked, and added if it cannot. `tools/nv/proofs/roster.ts:27`
  - [x] `roster.ts`'s `DETAILS` and `detailsFile`, and `collect.ts`'s `detailsProblems`, judged by the `comments` proof and `--comments`
  - [x] checked: an example could end with exit 1, but only its empty stdout was compared, so nothing pinned the diagnostic
  - [x] `// proof: compile-error` in `run.ts`: exit 1, a diagnostic on stderr, and the `.out` is the diagnostic with the path shortened; documented in `docs/examples/README.md`
- [x] `40-statements` is done first, as the pattern: each feature's facts sorted into its three homes, the chapter's unique code blocks made examples. `docs/reference/lang/40-statements.md:244`, `docs/examples/lang/statements/break-and-continue/about.md:1`
  - [x] `break-and-continue`: `details.md`, the chapter's two code blocks as examples 04 and 05
  - [x] `return`: `details.md`, and example 04, a `void` method's `return 0;` that does not compile
  - [x] `throw`: no `details.md`, since `about.md` has every fact, and example 04, an uncaught error ending with status 1
  - [x] `try-catch-finally`: `details.md`, examples 04 (`E0406`) and 05 (`finally` on `continue`), and the bare-`try` claim fixed in both chapters
  - [x] `catch-as-an-expression`: `details.md`, examples 04 (the result type) and 05 (`E0126`)
  - [x] `echo-print-unset-exit-yield`: `details.md`, and example 04 (`E0234`)
  - [x] `statement-forms-that-do-not-parse`: `details.md`, example 04 (`goto`), and `about.md`'s two wrong claims fixed
  - [x] the chapter's features before `break-and-continue`, from its top
- [ ] The other ten chapters, one per slice: `10-programs`, `20-types`, `30-expressions`, `50-classes`, `55-enums`, `60-iteration`, `70-errors`, `80-concurrency`, `90-attributes`, `95-testing`. `docs/reference/lang/10-programs.md:1`
  - [x] `10-programs`: six `details.md`, examples 04 for `doc-comments`, `names-and-casing` and `ending-a-program`, and five wrong claims fixed
  - [x] `20-types`
    - [x] `every-binding-has-a-type`, `numbers-bool-int-uint-float-decimal`, `text-string-and-bytes` and `array-t`
    - [x] `callable-classes-object-shapes`, `mixed`, `nullable-union-single-value-and-enum-case-types` and `void-never-self-static`
    - [x] `type-aliases`, `values-written-in-the-code`, `widening-without-as` and `the-conversion-operator-as`
      - [x] `type-aliases`
      - [x] `values-written-in-the-code`
      - [x] `widening-without-as`
      - [x] `the-conversion-operator-as`
    - [x] `property-keys`, `property-paths`, `strings-with-holes` and `narrowing`
      - [x] `property-keys`
      - [x] `property-paths`
      - [x] `strings-with-holes`
      - [x] `narrowing`
    - [x] `truthiness`, `parameters`, `properties-and-constants`, `qualifiers-tainted-and-secret` and `what-does-not-exist`
      - [x] `truthiness`
      - [x] `parameters`
      - [x] `properties-and-constants`
      - [x] `qualifiers-tainted-and-secret`
      - [x] `what-does-not-exist`
      - [x] `lang:types`' perf recorded after the chapter's last edit
  - [x] `30-expressions`, `50-classes` and `55-enums`
    - [x] `precedence-and-associativity`, `operators-that-do-not-parse`, `arithmetic` and `comparison-and-equality`
      - [x] `precedence-and-associativity`
      - [x] `operators-that-do-not-parse`
      - [x] `arithmetic`
      - [x] `comparison-and-equality`
    - [x] `logical-operators-and-truth`, `string-operators`, `and-the-ternary` and `assignment`
    - [x] the rest of `30-expressions`, then `lang:expressions`' perf recorded after the chapter's last edit
      - [x] `match` to `anonymous-functions`
      - [x] `arrays-in-expressions`
      - [x] `anonymous-objects`
      - [x] `refused-in-expression-position`
      - [x] `lang:expressions`' perf recorded after the chapter's last edit
    - [x] `50-classes`, then `lang:classes`' perf recorded after the chapter's last edit
      - [x] `declaring-a-class`, `properties`, `methods-self-static-and-parent` and `constants-and-class`
      - [x] `inheritance`, `interfaces`, `property-hooks` and `lateinit`
      - [x] `PropertyGuard` to `Parses`
      - [x] `objects-are-handles` to `what-a-class-cannot-declare`
      - [x] `lang:classes`' perf recorded after the chapter's last edit
    - [x] `55-enums`, then `lang:enums`' perf recorded after the chapter's last edit
      - [x] `declaring-an-enum` to `comparing-cases`
      - [x] `match-and-switch-over-an-enum` to `an-enum-inside-a-class`
      - [x] `what-an-enum-does-not-have` and `core-enums`
      - [x] `lang:enums`' perf recorded after the chapter's last edit
  - [x] `60-iteration`, `70-errors` and `80-concurrency`
    - [x] `60-iteration`: its chapter did not change, so `lang:iteration`'s perf is current
      - [x] `what-foreach-walks` and `the-two-interfaces`
      - [x] `generators` to `what-does-not-exist`
    - [x] `70-errors`, then `lang:errors`' perf recorded after the chapter's last edit
      - [x] `the-throwable-tree` to `throw`
      - [x] `try-catch-finally` to `recursion-depth`
      - [x] `fatal-errors-limits-no-catch-sees` to `inspecting-a-value`
      - [x] `lang:errors`' perf recorded after the chapter's last edit
    - [x] `80-concurrency`, then `lang:concurrency`' perf recorded after the chapter's last edit
      - [x] `no-colouring-i-o-just-waits` to `limit-and-deadline`
      - [x] `a-child-s-throw` to `isolates-spawn-script-and-await`
      - [x] `a-child-shares-nothing` to `what-does-not-exist`
      - [x] `lang:concurrency`' perf recorded after the chapter's last edit
  - [ ] `90-attributes` and `95-testing`

### 7. the syntax pages show each feature's about.md, details and examples

**Does:** every syntax page shows the `about.md`, details and examples of each feature it covers, and
repeats none of their text.

- [ ] `feature.ts` reads `lang` like the other areas, and the component the `Core` pages use renders a `details.md`. `website/src/lib/feature.ts:33`
- [ ] Each syntax page renders its covered features, keeps its own frame, *Use this when* and *Do and don't*, and loses its snippets for those features. `website/src/content/docs/syntax/control-flow/break-continue-return.mdx:1`
- [ ] The site lock hashes a `lang` feature's `about.md`, `details.md` and examples. `tools/nv/cmd/site.ts:306`

### 8. the chapters under docs/reference/lang are generated from about.md and details.md

**Does:** no chapter section is hand-written, and `nvs agent` prints the same text the website does.

- [ ] A renderer writes each chapter from its features' `about.md` and `details.md` in roster order, and a `--check` refuses a hand edit. `tools/nv/proofs/roster.ts:28`
- [ ] `nvs agent primer` and `show`, and `/llms-full.txt`, read the generated chapters, and their tests pass unchanged in shape. `crates/nvs-cli/src/agent.rs:187`

## Handoff

### State

Stage 1 is done but for one file. **Blocked on looper, not on this repo:** `docs/design.md` stays as
a five-line pointer, because looper's managed block in AGENTS.md links it and `looper lint` refuses a
hand edit inside the markers. Once looper's `agents` template stops linking it, delete the file, add
`design\.md` to `DELETED_DOC_CITE` (`tools/nv/cmd/rules.ts:107`) and tick stage 1's last sub-item.

Stages 2, 3 and 4 are done.

Stage 5 is done. AGENTS.md's project half is the priority ordering, § *Novis works without looper*
(copied verbatim from `main`'s 254ff50d1, so the merge is clean), § *The rules only this project has*,
§ *Text an end user reads*, § *In the loop* and the stack departures. Every `bun nv` instruction the
cut sections held is still named there or in `docs/agent/commands.md`: `peek`, `splice --patch`,
`affected --run` as the gate, `verify --fast -p <crate>` as the mid-work check, and the `looper wrap`
steps by hand. New homes: `commands.md` § *What the guards refuse, and the call to make instead*
(each refusal of `bun nv guard` and `looper guard`), § *Scratch files and worktrees*, and the side-goal
paragraph under § *The loop's hooks*; `conventions.md` § *A handoff*. Every citation of a removed
section or rule number is repointed, including `guard.ts`'s `cargo-p` message, which cited
"AGENTS.md rule 5" where the block's rule 5 is something else.

Stage 6's tooling is done. `details.md` is optional beside `about.md`, and the `comments` proof and
`bun nv proofs --comments` hold it to the sentence and dash bounds, with no heading and no code block
(`detailsProblems`, `tools/nv/proofs/collect.ts`). An example that declares `// proof: compile-error`
must end with exit 1 and a diagnostic, and its `.out` is that diagnostic with the path shortened to
the file name (`exampleOutput`, `tools/nv/proofs/run.ts`). `docs/examples/README.md` documents both.

`break-and-continue` is the pattern: the facts `about.md` lacks went to `details.md`, each checked on
`target/proof/nvs.exe`, and the chapter's two code blocks became examples 04 (a `switch` in a loop)
and 05 (a compile error). `about.md`'s closing sentence names the two. The three texts agreed on every
fact; `continue 1` inside a `switch` in a loop also continues the loop, which no text said. The
chapter and the syntax page keep their own copies until stages 7 and 8 replace them.

`return` and `throw` followed it. `return`'s `details.md` holds three facts the binary confirmed and
no text stated whole: `return;` in a non-`void` method is `E0822`, a path that reaches the end of
such a body is `E0739` (a `throw` ends a path too), and a required file's top-level `return` value is
what `require` gives. Its example 04 is the chapter's "`void` returning a value" as a compile error
(`E0403`). `about.md` lost "the one that answers". `throw` needed no `details.md`; its example 04 is
the chapter's uncaught throw (`// proof: exit 1`), and `about.md` gained its closing sentence. No
text disagreed with the binary.

`try-catch-finally` found the first disagreement. Both chapters said a `try` with neither `catch` nor
`finally` is accepted; the binary, the rule and the conformance case make it `E0242`. Both chapter
lines now say so (`40-statements.md:325`, `70-errors.md:232`), and `about.md` no longer says a `try`
needs "one or more" `catch` clauses, since `try { } finally { }` is valid. `details.md` holds the
rest, each run on the binary: an interface in a `catch`, a parent class first compiles and shadows
silently, `E0245` and `E0406`, a throw from a `catch` passing through `finally`, nested `finally`
order. The chapter's `E0406` block is example 04 and its `continue` loop example 05; its `Parse`
block is examples 02 and 03 already. A `return`, `break` or `continue` leaving `finally` (`E0250`,
`E0251`) is the errors chapter's fact, and goes to `lang:errors/try-catch-finally`'s `details.md`
when `70-errors` is done; that feature has its own `about.md` and examples, which overlap this one's.

The three sections after `throw` are done the same way, each fact run on the binary.
`catch-as-an-expression`'s `details.md` adds the precedence, the optional variable, `E0245`, the
union type and `E0401`, `E0126`, the enclosing `finally` and `W1006`. The chapter's "the binding is a
local of the enclosing function" is half true: an arm's variable clashes with an earlier local
(`E0406`), but a later declaration and a second arm may reuse the name. Its example 04 is the
chapter's `mixed` union and a `?int`, and 05 is the chapter's `E0126` block.
`echo-print-unset-exit-yield`'s `details.md` adds `E0234`, `E0413`, `exit` as an expression,
`E0445`, `E0446` and `E0448` for `yield from`; example 04 is the chapter's `unset($x)` block.
`statement-forms-that-do-not-parse` had two wrong claims in `about.md`: `[$a, $b] = …` does not
compile (each variable needs its type, as the chapter says), and a label, `declare` and the
alternative syntax get a general syntax error that names no replacement. Its `details.md` lists each
form's code, and example 04 is the chapter's `goto` block.

The first three sections of `40-statements` are done, and no text disagreed with the binary.
`expression-statements-blocks-and-declarations`'s `details.md` adds `E0406` for a second declaration
(in a block, or a second `for` loop's variable), a declaration in a loop body running every round,
and `E0301` for a variable declared in one `if` branch. That last one is the chapter's error block, now
example 04. `if-elseif-else` and `while-and-do-while` need no `details.md`: `about.md` already has every
fact, and examples 01 and 02 show the chapter's blocks. The chapter's `if (…): … endif;` error block
is `statement-forms-that-do-not-parse`'s example 05, since that feature owns the alternative syntax.
Its `details.md` now names all five `end…` keywords, each checked on the binary.

`40-statements` is done: `for`, `foreach` and `switch` closed it, and no text disagreed with the
binary. `for`'s `details.md` adds `E0125` (two declarations), `E0124` (a declaration mixed with
expressions), the condition list running in full on every test, and `E0406` for a second loop's
counter; example 04 is the chapter's `for (;;)` and 05 its `E0125` block. `foreach`'s `details.md`
adds mixed `var` and written types, `E0101`, the qualifier `var` keeps, a wider written type,
`E0444`, the parenthesised conversion and `E0443`, `E0414` for a literal subject, an `inout` loop
walking the array itself, and `E0406`; examples 05 to 08 are the chapter's generator, its wider type
with a converted subject, and its two error blocks. `switch`'s `details.md` adds `==`, `E0466`, and
a `default` in the middle falling through, which is example 04. The `switch` level for `break` and
`continue` stays `break-and-continue`'s fact.

`10-programs`' first five sections are done. `code-mode-and-html-mode` found a disagreement: the
chapter said the `;` before `?>` is optional in both tag forms, but the binary makes it optional in
`<?= ?>` alone and gives `E0101` for `<?nvs echo "a" ?>`; the chapter line now says so
(`10-programs.md:77`). Its `details.md` holds that, the shebang facts (`E0009`) and the escaping in a
request and in an html template, each run on the binary; examples 01 and 02 already show the
chapter's blocks, and the shebang script is `tools/cli`'s example. `doc-comments`' `details.md` adds
`E0323` (`@see` to nothing), `E0325` (`@example` outside `examples/` or `tests/`), `E0128`, `E0127`
for a `///` above a statement as well as a blank line or the end of the file, and `W1011`; example 04
is `@param` as a compile error. Its `about.md` said the tags go "at the end of the comment", but the
binary accepts text after a `@see` line, so it now says "at the start of a line".
`a-program-is-a-file-of-top-level-statements`, `a-complete-program-annotated` and `comments` need no
`details.md`: `about.md` has every chapter fact, and the examples show the code.

`10-programs` is done. Its last six features each have a `details.md`, every fact run on the binary,
and five claims were wrong. `names-and-casing`: `about.md` and example 02 said only a name's first
letter is checked, but an underscore is refused too (`$line_count` is `E0112`, enum case `GOLD_RANK`
`E0110`); its `details.md` lists `E0110` to `E0114` and `E0122`, and example 04 is the chapter's
`order_line` block. `namespaces-and-use`: example 02 said a `use` covers the whole file, but it covers
only the code below it; `details.md` adds `E0243`, `E0233`, `E0240`, `E0212`, `E0238`, `E0217` and
the enum or alias in a class body (`E0133`). `require`: the chapter said a computed path is read at
run time, but any path not fixed while compiling throws `RuntimeError` when reached, even to a file
that exists (`10-programs.md:264`); class constants joined with `.` count as fixed. `autoload`: both
the chapter and `about.md` said a missing class's error lists the searched roots; it is a plain
`E0303`, and `nvs check --autoload-map` shows the roots (`10-programs.md:301`). File paths: the
chapter said `'data/' . $name` throws, but it is `E0840` while compiling, as `about.md` said
(`10-programs.md:325`). `ending-a-program`: `about.md` was rewritten to § *Text an end user reads*,
its `details.md` has the status byte, `exit("…")` adding no newline and `E0228`, and example 04 is the
chapter's `exit(3)` block (`// proof: exit 3`). A `catch` does not stop `exit` and `finally` does not
run, which no text said.

`20-types`' first four features are done, each fact run on the binary. `every-binding-has-a-type`
found a disagreement in three texts: the chapter and `try-catch-finally`'s `details.md` said two
`catch` clauses in one function need two names, but they may share one when they catch the same
type, and only clauses of different types are `E0406`. All three now say so
(`20-types.md:36`, `try-catch-finally/details.md:9`, its example 04's comment). Probing that found
a compiler panic: a plain local, then a `catch` variable of the same name and type, passes the
checker and panics in lowering. It is the gap record `data/gaps/nvs-ir/a-catch-variable-that-reuses-a.json`,
owned by `release-hardening`. Its `details.md` has `E0401`, `E0301` both ways, `E0414` for an empty
or mixed array and the `catch` rule; examples 04 and 05 are the chapter's two type-error blocks.
`numbers-bool-int-uint-float-decimal` and `text-string-and-bytes` had `about.md` in the essay voice
and were rewritten to § *Text an end user reads*. Their `details.md` have `E0407`, `E0455`, `E0716`,
`E0708`, the overflow messages and `true as string` being `"1"`; and `E0482` (for `bytes` too),
`E0707`, `E0715` and `E0716`. Numbers' example 04 is the chapter's `E0455` block, and text's is its
`fromHex` block with the `RuntimeError`. `array-t`'s `details.md` has the appended key after a gap,
`E0401` for a literal and an append, the missing key's `RuntimeError`, `E0707` and `E0467`; example
04 is the chapter's key and copy block. The 20-types chapter changed, so its `lang:types` benches
are perf-stale until the chapter's last edit, as the playbook says.

The next three `20-types` features are done, each fact run on the binary, and no text disagreed with
it. `callable-classes-object-shapes`' `details.md` has `E0418`, a property read through `object`
being `mixed`, `as Cell` throwing and `as ?Cell` being `E0473`, `E0708` and the run-time error for
`as class<T>`, `E0496` at all three class-reference sites, `E0466`, `E0134` and `E0401` for a shape;
example 04 is the chapter's `object` lines and 05 the bare shape. Probing found a second compiler
panic: `echo` or `.` with a `class<T>` variable passes the checker and panics in lowering, now the gap
record `data/gaps/nvs-ir/a-class-reference-in-echo-or.json`, owned by `release-hardening`. `mixed` and
`nullable-union-single-value-and-enum-case-types` had `about.md` in the essay voice, now rewritten.
`mixed`'s `details.md` has what each operation throws on the wrong type (a method call on a
non-object throws `LogicError`, the rest `RuntimeError`) and `"5" == 5` being false; examples 04 and
05 are the chapter's two blocks. Nullable's `details.md` has `E0401`, `E0716`, enum numbering, `E0708`
and `iterable`; examples 04 to 06 are the chapter's three blocks. Arithmetic on an unnarrowed
`int|string` parameter compiles and gives `mixed`, which no text states and this session did not
probe further.

`void-never-self-static` and `type-aliases` are done, each fact run on the binary.
`void-never-self-static` found two wrong claims, both in the chapter and in the syntax page
`website/src/content/docs/syntax/functions/declaring.mdx`. A call to a `never` method does end the
caller's path, as `rule:statements/a-body-never-falls-off-its-end` says, unless a subclass overrides
it (`20-types.md:335`). `self` also works as a parameter type, so it is not return-only. Its
`about.md` was in the essay voice and is rewritten, and so are examples 01 and 02's comments. Its
`details.md` has `E0742`, `E0707`, `E0401`, `E0739`, `E0822`, the override case, `E0303`, `E0741`
and `E0321`. Example 04 is the chapter's `echo` of a `void` call (`E0707`), and example 05 is a
caller with no `return` after a `never` call. Probing found a missing diagnostic: a `void` or
`never` property is not `E0742`, and fails only on its default or its constructor. That is the gap
record `data/gaps/nvs-types/a-void-or-never-property-passes.json`, owned by `release-hardening`.
`type-aliases` had `about.md` and example 01's comment in the essay voice, now rewritten, and no
text disagreed with the binary. Its `details.md` has `E0110`, use above the declaration, `E0233`,
`E0133`, `E0303` in a child class, `E0307`, an alias of an alias, `E0310` and `E0304`. Example 04 is
the chapter's `?User` and union block, and 05 its `E0307` block.

`values-written-in-the-code` and `widening-without-as` are done, each fact run on the binary, and
no text disagreed with it. `values-written-in-the-code` owns the html template too (no other feature
does). Its `details.md` has the escapes, `E0431` for an escape that is not valid UTF-8 (the
chapter's "octal `\0` through `\777`" omits that), `E0011` in every form, the bare interpolation
reaching one level, `"${name}"` printing as written, `E0707` and `E0412` in a string, `E0433`, a
duration's `==` being identity, `E0117`, and the template's `W1012`, `E0010`, `E0790`, `E0710`,
`E0319` and `E0417`. Examples 05 to 07 are the chapter's escapes block, its durations block and its
`.` on markup error block. The `html"…"` error block is not an example: it prints four cascading
errors. `widening-without-as` had `about.md` and examples 01 and 02's comments in the essay voice,
now rewritten. Its `details.md` has the untyped literal, the `ArithmeticError` above 2^53 (also for
9007199254740994, which a `float` stores exactly), single-value types, each `E0401` that needs `as`,
and the shape field conversion. Example 04 is an anonymous object at a shape, 05 the chapter's
`int` into `uint` block. A local typed with a bare shape is `E0134`, so a probe names the shape
with `type` first.

`the-conversion-operator-as` is done, each fact run on the binary, and no text disagreed with it.
Its `details.md` has the two error classes, which text converts to a number (`"+5"` and `".5"` do,
`"1_000"` and `"inf"` do not), `float` and `"1e3"` to `decimal`, `bool` and the truthiness values,
`bytes`, enum cases, `E0708` for a pair with no rule, `mixed` giving a `tainted string`, the array
check (elements are not converted from text: `["1"] as array<int>` throws, which the chapter's table
row leaves unclear), `E0709`, `E0473`, and `class<T>` at compile and run time, a string always being
the full name. Examples 04 to 08 are the chapter's five blocks but `(int)$n`, which belongs to
`what-does-not-exist`'s slice. Example 01's comments were in the essay voice, now rewritten. The
chapter's `property<T>`, `propertyPath<T>` and shape rows are those features' facts. Probing found
a third compiler panic: `$c is class<Dog>` passes `nvs check` and panics in lowering, now the gap
record `data/gaps/nvs-ir/an-is-test-against-a-class-reference.json`, owned by `release-hardening`.
The chapter did not change in this slice.

`property-keys` and `property-paths` are done, each fact run on the binary, and no syntax page covers
them. `property-keys`' `about.md` said a key cannot write to a class with any `readonly` property, but
a `private readonly` one does not stop the write; it now says "public `readonly`", as the chapter
does. Its `details.md` has `E0799`, `E0405` for a literal and at a parameter, the `as` error for a
private name, `E0235`, the union read type, the two write checks (`E0401` for a value no property
takes, every property for a `tainted` or `secret` value), `E0782`, the parent's key on a child, and
equality. Examples 05 to 07 are the chapter's three blocks. Probing found a bug: a `public static`
property's name converts to a key, and a read through it throws "`User` has no field `count`". That
is the gap record `data/gaps/nvs-types/a-property-key-names-a-static.json`, owned by
`release-hardening`. `property-paths`' `details.md` has `E0405` for a wrong part and for a part after
a non-object, the run-time messages (an empty part too), `E0235` for `$obj->$path`, and equality.
Example 04 is the chapter's `as` error, extended to each message, and 05 a wrong literal path. The
chapter did not change.

`strings-with-holes`, `narrowing`, `truthiness` and `parameters` are done, each fact run on the
binary. `strings-with-holes` found a disagreement: the chapter said a wrong hole written in the code
is `E0405`, but for a set of strings it is `E0861`; the chapter line now says so
(`20-types.md:1004`). Its `details.md` adds a hole's spaces, `E0401` for a literal at a local, the
run-time messages and the tainted case; example 04 is a wrong hole (`E0405`). `narrowing` needed
no fix. Its `details.md` adds `E0477` after the false branch of `is`, the false branch of `==` on a
set keeping the full type, `switch (true)`, where narrowing ends (`E0459`) and `?->`; examples 04
and 05 are the chapter's `&&`/`||` block and its `E0459` block. `truthiness` had `about.md` and
examples 01 and 02's comments in the essay voice, now rewritten; its `details.md` adds enum cases,
`mixed`, `-0.0`, `empty`/`isset`, `E0466` for an int `match (true)` label and the `bool` positions;
example 04 is the chapter's `as bool` block. `parameters` found a wrong claim in four texts: the
chapter (`20-types.md:1227`), `30-expressions.md:472`, the syntax page `syntax/functions/parameters.mdx`
and `rule:statements/an-inout-argument-is-a-local` all said a property cannot be passed `inout`. The
binary and the conformance cases (`a-by-reference-argument-is-written-inout-at-both-ends.nvst`,
`an-inout-argument-names-storage-that-outlives-the-call.nvst`) accept an instance property, which
`check_inout_arg` (`crates/nvs-types/src/expr/args.rs:1361`) does on purpose. All four now say so;
the rule's id is kept and its title changed. Its `details.md` has `E0451`, `E0402`, `E0488`, `E0486`,
`E0487`, the `E0439` list, `E0440`, `E0713`, `E0237` and `E0493`; example 04 is `E0713`. Probing found
a missing diagnostic: a variadic parameter that is not the last compiles and acts as a plain one,
now the gap record `data/gaps/nvs-types/a-variadic-parameter-that-is-not.json`, owned by
`release-hardening`. `website/public/llms-full.txt` is rendered again with the chapter changes.

`20-types` is done: its last three features closed it, each fact run on the binary, and no text
disagreed with it. `lang:types`' perf is recorded after the chapter's last edit.
`properties-and-constants`' `about.md` was in the essay voice, now rewritten. Its `details.md` has
`E0101` for an untyped property, what a default may be and `E0472`, `E0409` in the constructor and
for a `static` property, `E0246`, `E0401` at the declared type, and `E0792` on every read of a
constant whose value does not fold (another class's constant, an enum case, `self::B + 1`, `.`).
Example 04 is a computed default (`E0472`). Probing found two gaps, both owned by
`release-hardening`. A `private` or `protected` constant can be read from anywhere
(`data/gaps/nvs-types/a-private-or-protected-class-constant.json`). A constant whose value does not
fit its type compiles until read, and is then `E0792`
(`data/gaps/nvs-types/a-class-constant-whose-value-does.json`).
`qualifiers-tainted-and-secret`'s `about.md` and examples 01 to 03's comments were in the essay voice,
now rewritten. Its `details.md` has `E0102` for a qualifier on another type or in the wrong order,
each `E0401` sink, `as string` keeping the qualifier, `E0790`, `E0422`, `E0724`, `E0791`, the array
literal that drops `secret`, and `revealBytes`. Examples 04 to 07 are the chapter's six error blocks:
`Core\Debug::dump` and `Core\Json::encode` share 07 with the interpolated `echo`.
`what-does-not-exist`'s `about.md` and examples 01 to 03's comments were rewritten, and `about.md` no
longer says each diagnostic names the replacement: `gettype`'s `E0320` names none. Its `details.md`
has `E0225`, `E0320`, `E0208`, `E0482`, `E0716`, `E0715`, `E0466`, `E0230` and `E0418`. The
chapter's `(int)$n` block moved from the `as` section into this one. Examples 04 to 06 are that
block, the `gettype` and `settype` blocks, and a string in arithmetic.

`looper lint` failed on a stale managed block, so `looper sync` brought AGENTS.md's `agents` block to
v3. It adds § *looper only orchestrates* and still links `docs/design.md`.

The sweep's red items are not this branch's: run 39108526796 ran on `main` at 1a78b49ef, as earlier
sessions found, and the red bench `benches/members/core/Ldap-Connection/delete.nvs` belongs to the
`ldap` work on `main`. `bun nv site --check` also fails on pages this goal has not touched.

`30-expressions`' first four features are done, each fact run on the binary.
`precedence-and-associativity`'s `details.md` has the full order as a list, `as`, `!` with `is`,
`|>`, `new …->m()`, `clone`, the three `W1022` cases and the expression `catch`; example 04 is the
chapter's block, extended to `!`/`is`, `|>` and `new`. `operators-that-do-not-parse`'s `about.md`
said every error names the replacement, but a backtick and `&$x` give a general syntax error; its
`details.md` lists each form's code, and example 04 is the chapter's `===` block. `arithmetic` found
two chapter errors, both against `rule:types/arithmetic`, which the binary agrees with: `%` works on
`decimal` (the chapter said integers only), and `**` on a `decimal` is `E0455`. A `decimal` division
by zero throws `ArithmeticError` with the range message, not `Division by zero`. The chapter's table
(`30-expressions.md:101`) and the syntax page's `%` row now say so. Its `about.md` was rewritten (it
said both sides must be the same kind, and example 01 adds an `int` and a `float`); examples 04 to 06
are the chapter's `E0407`, unary and bitwise blocks. `comparison-and-equality` found one wrong claim
in four texts: `<=>` on two objects returns `compareTo`'s value unchanged, as
`rule:classes/ordering-lowers-to-compare-to` says, not `-1`, `0` or `1`. The chapter
(`30-expressions.md:212`), `about.md`, example 03's comment and the syntax page now say so. Its
`details.md` has each `E0466`, `E0715` and `E0411` pair, exact `int`/`uint` comparison, arrays,
`mixed`, and a `?int` with `<` compiling and throwing `RuntimeError` on `null` (the union is answered
from the tag, by design in `crates/nvs-ir/src/ir.rs:2761`); examples 04 and 05 are the chapter's two
error blocks. Probing found a compiler bug: `true < 3` and `1 < 2 < 3` pass the checker and stop at
`E0901`, now the gap record `data/gaps/nvs-types/ordering-a-bool-against-a-number.json`, owned by
`release-hardening`. `llms-full.txt` is rendered again.

The next four `30-expressions` features are done, each fact run on the binary, and the chapter did
not change. `logical-operators-and-truth`'s `details.md` has `E0226` with `!=` for `xor`, the `bool`
result, `E0718`, the values easy to get wrong (`"0.0"`, `NAN`, `-0.0`), nothing kept after the
expression (`E0459`), and where `W1023` is given: the whole condition of `if`, `while`, `? :` and
`?:`, never an operand of `&&`, `||` or `!`. Narrowing it there found a wrong claim in two texts:
`narrowing`'s `about.md` and `details.md` said a property is never narrowed, but a path of `readonly`
properties is, as `rule:types/narrowing` says; both now say so. The syntax page's list of false
values (`syntax/operators/comparison.mdx`) now has the empty `bytes` value. `string-operators`'
`details.md` has `E0707`, `E0412`, `E0710`, a `mixed` array's `RuntimeError`, `+` binding before
`.`, `.=` on an `int` (`E0401`) and on a `null` `?string`, and the tainted and secret results;
`about.md` no longer says every error names a replacement (`void`'s does not). `and-the-ternary`'s
`details.md` has the result types, `W1022`, `??=`'s value, `E0105` for `??*=`, the `uint` `??-=`
`ArithmeticError`, `?->` not running its arguments and `E0479`; example 04 is the chapter's `??-=`,
`??.=` and nested ternary lines, 05 its `E0479` block. `assignment`'s `details.md` has the twelve
compound forms, each `E0401`, `E0706`, `E0716` on a `?int`, the run-time errors, a missing key's
`RuntimeError`, `E0105`, `E0782`, `E0301` and `if ($a = 0)`; example 04 is the chapter's
compound-operator block, 05 `/=` on an `int`. Probing found two gaps, both owned by
`release-hardening`: a static property read through an instance (`$p->x`, `$p?->x`) passes
`nvs check` and stops at `E0901` (`data/gaps/nvs-types/a-static-property-read-through-an.json`),
and `$this = new P()` compiles and rebinds `$this`
(`data/gaps/nvs-types/an-assignment-to-this-compiles-and.json`).

`match`, `is-new-clone-throw-print-exit-isset-empty` and `calls` are done, each fact run on the
binary, and no text disagreed with it; the chapter did not change. `match`'s `details.md` has the
subject running once, arms in order stopping at the first match, `E0466` (a `null` arm and an `int`
condition in `match (true)` too), the union result type and `E0401`, narrowing in a `match (true)`
arm, `E0476` and the `LogicError` message; example 04 is the chapter's block, 05 an `E0466` arm. The
second feature's `details.md` has `is` on every type, `E0496` after `is` and `new`, `E0785`, `E0740`,
`E0781`, `E0780`, `print` with `E0707`, `isset`'s `E0498`, `E0301` and `E0459`, and `empty` on a
call; its `about.md` said "the compiler knows" and is rewritten; examples 04 and 05 are the chapter's
two blocks. `calls`' `details.md` has written-order evaluation of named arguments, `E0778`, `E0489`,
several `...` arguments with the keys kept, a `callable`'s `mixed` result and `LogicError`s, and a
`void` call's `E0401` and `E0707`; it leaves `E0402`, `E0486` to `E0488`, `E0439` and `E0713` to
`parameters`' `details.md`. Its `about.md` and examples 01 to 03's comments were in the essay voice,
now rewritten; example 04 is the chapter's block, 05 an `E0489` call. Probing found three gaps, all
owned by `release-hardening`: a `void` call at a `mixed` local or as a `match` arm stops at `E0901`
(`data/gaps/nvs-ir/a-void-call-at-a-mixed.json`), `$u == -1` on a `uint` throws `ArithmeticError`
(`data/gaps/nvs-types/a-negative-literal-compared-with-a.json`), and a second `default` in a `match`
or a `switch` compiles (`data/gaps/nvs-types/a-match-with-two-default-arms.json`).

`the-pipeline-operator` and `anonymous-functions` are done, each fact run on the binary, and no
text disagreed with it; the chapter did not change. The pipeline's `details.md` has where a step
ends (`$n |> $_ * 2` is `($n |> $_) * 2`, and `$n |> 10 - $_` is `E0129`), the step forms, a deep
`$_`, `E0129` to `E0131` with their cases, the subject running once, and `!` running after `|>`;
examples 04 to 06 are the chapter's four blocks. `about.md` and example 01 said `$_` "marks" the
place, now rewritten. `anonymous-functions`' `details.md` has `E0222`, `E0223`, `E0210`, `E0493`,
`E0237`, `E0808`, `E0450`, `E0401`/`E0403`, a captured copy restarting on each call, `E0301` for the
function's own variable, the named `fn fact(…)` (only the syntax page had it) and `E0320`, `E0779`,
method references, the `bindTo` message and `E0402`; example 04 is the chapter's block, 05 a named
function, 06 the `function` block. Probing found three gaps, owned by `release-hardening`: a default
on a `fn` parameter is never used
(`data/gaps/nvs-types/an-anonymous-function-parameter-default-is.json`), a `...` parameter on a `fn`
is a plain one (`data/gaps/nvs-types/a-variadic-parameter-on-an-anonymous.json`), and `echo` of a
callable compiles and throws naming `Script$fn0`
(`data/gaps/nvs-types/echo-of-a-callable-compiles-and.json`).

`30-expressions` is done: `arrays-in-expressions`, `anonymous-objects` and
`refused-in-expression-position` closed it, each fact run on the binary. The chapter did not change,
and `lang:expressions`' perf is recorded. `arrays-in-expressions`' `details.md` has `E0414`,
`E0434`, `"08"`, the next key after a negative one and after `unset`, `E0481`, the spread's later key
winning and `E0401`, writes creating inner arrays, a nested read's error, `isset` on a `null` value,
`E0700`, destructuring by key (extra entries skipped, mixed forms, a variable key), `E0101`, `E0102`,
`E0482`, no pattern in `foreach` and `E0230`; examples 04 to 06 are the chapter's three blocks.
`anonymous-objects`' `details.md` has field-name forms (`E0101`, no spread), `E0494`, `E0117`,
`E0401` on a write, a function in a field and `E0477`, the run-time error for a missing or optional
field, `isset`, a class at a shape, `clone`, JSON sorted by name, and `echo` throwing; its
`about.md` and examples 01 and 02 said a shape or `==` "asks", now rewritten. Examples 04 and 05
are the chapter's two blocks. `refused-in-expression-position`'s `about.md` said every error names a
replacement, but a backtick is a plain `E0001`; its `details.md` lists each form's code and help,
and example 04 is the chapter's `$$name` block. Example 03 named PHP twice, now rewritten. Probing
found two gaps, owned by `release-hardening`: type arguments after a built-in type name
(`iterable<int>`, `int<int>`) lose the method body and give `E0786` or `E0823`
(`data/gaps/nvs-syntax/type-arguments-after-a-built-in-type.json`), and a destructuring variable
already declared with the same type, or written twice in one pattern, compiles and overwrites it
(`data/gaps/nvs-types/a-destructuring-variable-that-is-already.json`). A `yield` used as a value in
a method with no other `yield` is `E0445`, not `E0448`; the rest of `details.md` names `E0448`.

`50-classes`' first four features are done, each fact run on the binary, and no text disagreed with
it; the chapter did not change. `declaring-a-class`' `details.md` has `E0122` for each member kind,
`E0114`, `E0110`, `E0471` (a `private` member in a subclass, a `private` constructor), `E0233` in a
class body, a method or an `if`, `E0402`, `E0405`, the inherited constructor, `E0722` and `new A`;
examples 04 and 05 are the chapter's no-visibility and no-constructor blocks. `properties`'
`details.md` leaves the defaults, `E0472` and `E0101` to `lang:types/properties-and-constants`'
`details.md`, and has `+=` not counting as the assignment, a `?T` starting as `null`, `E0248`, the
early `return`, `E0428`, `E0853`, `E0427`, `E0249`, each `E0782` form, `E0413`, the shared static
value, `E0499`, `E0309` and `E0471`; `about.md`'s list of defaults gained `bool` and a class
constant. Examples 04 to 09 are the chapter's six blocks that no example showed.
`methods-self-static-and-parent`' `details.md` has the anonymous function reaching `private`
members, a static method called on an object, `E0496`, `E0778`, `E0779`, `E0702`, `E0303`, `E0410`,
`self` losing the subclass (`E0405`), `E0741`, `new self()` in a static method, and `E0834`;
example 04 is the chapter's `Acc` block, 05 an `E0741` method. `constants-and-class`' `details.md`
has `E0113`, `E0106`, `E0309`, `E0105`, `E0700`, `E0496`, `E0833` with a narrower type allowed, an
interface constant read through `static::`, `E0832` (also for a `?int`), `E0831`, and `E0702`;
example 04 is an `E0832` block. Probing found four gaps, owned by `release-hardening`: a member
declared twice passes `nvs check` and stops at `E0901` or exits 5 with no output
(`data/gaps/nvs-types/a-class-member-declared-twice-passes.json`), a constructor reading a property
before assigning it reads `0` (`data/gaps/nvs-types/a-constructor-reads-a-property-before.json`),
`final` on a constant is accepted and does nothing (`data/gaps/nvs-syntax/final-on-a-class-constant-is.json`),
and a `static` constructor compiles and `new` runs it
(`data/gaps/nvs-types/a-static-constructor-compiles-and-new.json`).

`inheritance` and `interfaces` are done, each fact run on the binary, and no text disagreed with it;
the chapter did not change. `inheritance`'s `details.md` has `extends A, B`, `E0303` and `E0305`,
`E0410` on every path, a second `parent::constructor` call, `E0309` when the parent has none, `E0471`,
`parent::` reaching a grandparent, a default value coming from the variable's declared type, `E0449`,
`E0785`, `new static()`, `E0786`, `E0106`, `E0835`, `E0783`, `E0784`, `is` and the `class<T>` sites
(`E0496`, `E0235`, `E0794`, `E0708`). Examples 04 to 11 are the chapter's eight blocks that no example
showed. `about.md` said "marks" twice and example 01 "answers", now rewritten. `interfaces`'
`details.md` has `E0254`, `E0246`, the constant read with `static::`, `E0435`, `E0313`, the `static`
method, `E0449`, `E0303` both ways, `E0305`, a parent class winning over a default method, `E0859`,
each `E0720` form, the forward covering only methods without a body and reading the property on
every call, the `lateinit` `LogicError`, `E0721` and `E0227`. Examples 04 to 08 are the chapter's
five blocks that no example showed. Probing found two gaps, owned by `release-hardening`. **A method
override with another signature compiles, and a call through the parent reads the wrong type**: an
`int` method overridden by a `string` one gives the string's address as a number, and other
parameter types exit 5. That is a type confusion in safe code
(`data/gaps/nvs-types/a-method-override-with-another-signature.json`). An `abstract` method with a
body compiles and acts as an ordinary method (`data/gaps/nvs-types/an-abstract-method-with-a-body.json`).

`property-hooks`, `lateinit`, `propertyguard` and `propertyobserver` are done, each fact run on the
binary. One claim was wrong in three texts: the chapter, the observer's `about.md` and the syntax page
`syntax/classes/property-hooks.mdx` said a throwing `onPropertySet` fails or stops the write, but the
value is already stored, as `rule:classes/property-observer-pipeline` says ("an observer's throw
arrives after the store"). All three now say so (`50-classes.md:1105`). `property-hooks`'
`details.md` has the block `get`, a `set` with no parameter using `$value`, `E0401`, no default
(`E0101`), `E0843`, `E0254`, `E0787`, `E0478` in the class's own methods too, hooks running from
other methods and for `+=` and `++`, `E0413`, inheritance and redeclaring, `isset`, and two `get`
hooks recursing; example 04 is the chapter's `E0478` block. `lateinit`'s `details.md` has the read
before a write compiling only with `LateInit` (`E0428` without), `E0449`, `isinit`, `+=`, one call
setting several properties, a subclass property, the in-call read's message, and a second write in
`initProperty` running the pipeline; example 04 is the chapter's block, with the "still not
initialized" message. `propertyguard` and `propertyobserver` need no new example, since examples 01
and 02 show their chapter blocks; their `details.md` have a default not being a write, `static`,
`mixed` (`E0401`), nested keys, `+=` and `++`, guard recursion, a throwing unset, `isset` and
interpolation calling `onPropertyGet`, a `get` hook's order, and the `tainted string`. Probing found
six gaps, all owned by `release-hardening`, two of them safety bugs. **A hooked property is not
checked for initialization, and a read of an unwritten object or `string` one exits 5**
(`data/gaps/nvs-types/a-hooked-property-is-not-checked.json`). **A `set` hook whose parameter type is
not the property's compiles, and a write exits 5** (`data/gaps/nvs-types/the-parameter-type-of-a-set.json`).
**A promoted constructor parameter runs no guard and no observer**, so `constructor(public int
$balance)` passes a guard that always throws (`data/gaps/nvs-ir/a-promoted-constructor-parameter-runs-no.json`).
A hook on a `static` property never runs (`data/gaps/nvs-types/a-hook-on-a-static-property.json`),
two `get` hooks stop at `E0901` (`data/gaps/nvs-types/a-property-with-two-get-hooks.json`), and
`??=` on a property that has a value calls the guard (`data/gaps/nvs-ir/a-coalescing-assignment-to-a-property.json`).
`initProperty(int $name)` compiles too, which is the override-signature gap already recorded.

`stringable`, `comparable`, `parses` and `objects-are-handles` are done, each fact run on the binary,
and no text disagreed with it; the chapter and the syntax page did not change. `stringable`'s
`details.md` has `E0412` (a `toString` without `implements` too), `E0449`, `E0111` for
`__toString`, `E0401`, `E0466`, `Stringable` as a type, a `null` giving the empty string and a throw
from `toString`; example 04 is the chapter's error block. `comparable`'s has `E0411` for each case
(two classes, a subclass and its parent, a number, a `?V`), `E0449`, one call per operator and `==`;
example 04 is the chapter's error block. `parses`' has `E0449`, `E0404` for a return type other than
`static`, the `tainted` property (`E0401`), `E0752` with the list of capture types, and the
`404`/`400` split, which `a-segment-the-class-refuses-never-reaches-the-handler.nvst` pins; examples
04 and 05 are the chapter's route block and its `E0752` block. `objects-are-handles`' has the
subclass clone, a `private` constructor, objects inside a cloned array being shared, `!=`, `E0466`
and `== null`; `clone` of a `null` `?T` throwing `LogicError` went to
`is-new-clone-throw-print-exit-isset-empty`'s `details.md`, which owns `clone`. Probing found a
safety bug: **an ordering of two objects is checked on their static class alone**, so two
`Comparable`-typed operands of different classes, or a subclass whose `compareTo(self)` gets a parent
object, read the wrong type or exit 5 (`data/gaps/nvs-types/an-ordering-of-two-objects-passes.json`,
owned by `release-hardening`). The override-signature gap now names the built-in interfaces: a
`toString(): int` stops `echo` with an internal error, and `parse(string $s)` drops the `tainted`
qualifier.

`50-classes` is done: `nullable-objects-and`, `object-the-top-of-every-class-type` and
`what-a-class-cannot-declare` closed it, each fact run on the binary, and `lang:classes`' perf is
recorded. One claim was wrong in three texts: the chapter, `what-a-class-cannot-declare`'s `about.md`
and the syntax page `syntax/classes/objects.mdx` said an `enum` is the one type a class body may
declare, but a `type` alias may be declared there too, as `E0233`'s help and
`declaring-a-class`'s `details.md` say. All three now say so (`50-classes.md:1422`). The nullable
feature's `details.md` has `E0459`, the `?T` result and `E0401`, each `?->` covering one step (so
`$b?->inner()->label` is `E0459`), a `void` call and `isset`; the rest of `?->` stays in
`and-the-ternary`'s `details.md`. `narrowing`'s gained that an assignment narrows nothing. The
`object` feature's `details.md` has `E0477` (through `?->` too), `E0401` for a non-object, `null` and
a class-typed parameter, the `mixed` read, and the run-time checks of a write; its `about.md` said
"knows" and "opens", now rewritten. `Core\Program::implementing` is the attributes feature's fact.
`what-a-class-cannot-declare`'s has `E0111`, `E0114`, `E0112`, `E0244`, `E0420` and `destructor`
being an ordinary method; example 03 named PHP twice, now once. The new examples are the chapter's
two error blocks (`what-a-class-cannot-declare` 04 and 05), `->` on a `?Box` and a method call
through `object`. Probing found a safety bug: **a `private` or `protected` property is read and
written from outside its class through an `object` or `mixed` variable**, which breaks every
class's encapsulation (`data/gaps/nvs-runtime/a-private-or-protected-property-is.json`, owned by
`release-hardening`).

`55-enums`' first four features are done, each fact run on the binary. One claim was wrong in two
texts: the chapter and `a-case-is-its-integer`'s `about.md` said `as int` is the only conversion out
of an enum, but `as bool` compiles and is always `true`, as `rule:enums/truthiness` implies. Both now
say so (`55-enums.md:102`). `declaring-an-enum`'s `details.md` has the literal forms and `E0436`,
`E0437` for a value out of range and for counting past the largest number, a trailing comma, an
empty enum, equal cases, `E0110`, `E0239`, `E0220`, `E0218`, `E0219`, `E0438`, `E0304`, `E0233` and
`E0309`; examples 04 to 07 are the chapter's counting block and its three error blocks (07 with no
values, so it gives one error). `a-case-is-its-integer`'s has `E0707`, `E0434`, `E0708` for each
type, `as bool`, `E0416`, `E0415` (also `-1 as Level`, since `as` runs before `-`), JSON and
`dump`, and a case in `mixed` keeping no enum; example 04 is the `as string` block.
`from-an-integer-back-to-a-case`'s has each failing step: a `float` with a fraction and a negative
number for a `uint` enum throw `ArithmeticError`, which is not a `RuntimeError`; it also has `null`,
`" 1"` and `"2.0"`, `E0708` for a `bool`, and `E0309` for `from`/`tryFrom`. No new example: 01 and 02
show the chapter's block. `comparing-cases`' has `E0466` for each pair and for `null`, `E0715`, and
`==` in `mixed` being by number across enums and against an `int`, which `rule:enums/representation`
states and declines to change; example 04 is the chapter's error block. Probing found two gaps,
owned by `release-hardening`: a case declared twice compiles and the later one wins
(`data/gaps/nvs-types/an-enum-case-declared-twice-compiles.json`), and `new` on an enum passes the
checker and stops at `E0901`, or gives "expected `M`, found `M`"
(`data/gaps/nvs-types/new-on-an-enum-passes-the.json`). `llms-full.txt` is rendered again.

The next four `55-enums` features are done, each fact run on the binary, and no text disagreed with
it; the chapter did not change. `match-and-switch-over-an-enum`'s `details.md` has no `default`
needed when every case has an arm, `E0466` for another enum's case, a number and `null` (in a
`case` too), an arm not narrowing the subject (`match (true)` does), a `switch` with no matching
`case` running nothing, an outside label on a smaller-type subject, and a `mixed` subject matching
by number; what `match`'s own `details.md` says is left there. Example 01 said `default` "answers",
now rewritten. `an-enum-as-a-type`'s has each `E0401`, `E0403`, `var` inference, `E0409` for a
property with no default, a parameter default, `?Suit` and JSON; `E0434` stays in
`a-case-is-its-integer`'s. Its `about.md` said a number "cannot reach" the type, now rewritten.
`a-union-of-cases-is-a-narrower-type`'s has `E0401` and `E0403`, a single case, cases of two enums,
what narrows (`==` on the whole enum too; not `||`, `!=`, `else` or a `match ($m)` arm), `is`, `as`
from a number (`E0470` for a literal), the nullable forms, an alias, and `==` with an outside case
compiling and being `false`; example 04 is the chapter's error block. `an-enum-inside-a-class`'s has
`E0133`, `E0309` and `E0303` in a subclass or an implementing class, `E0135`, `E0212`, `E0304` for
an enum, constant or alias of one name, `E0220`, `E0233`, the class's enum winning over a top-level
one of the same name, `::class` and two classes' enums being disjoint (`E0466`); examples 04 to 06
are the chapter's three error blocks. Probing found no new gap.

`55-enums` is done, and `lang:enums`' perf is recorded after the chapter's last edit. One claim was
wrong in two texts: `what-an-enum-does-not-have`'s `about.md` and example 01's comment said the
compiler checks a `match` against every case, so a case added later is a compile error. The binary
compiles a `match` with a missing arm and throws `LogicError` when the case reaches it, as
`match-and-switch-over-an-enum`'s `details.md` already said; both now say so. The chapter line on
`as uint` now says it is for a `uint` enum (`55-enums.md:408`). That feature's `details.md` has
`E0405` for a property or method on a case, `E0309` for `from`, `tryFrom`, `cases`, a constant and a
static property, `E0220` for `extends`, `E0101` for `abstract` or `final`, `E0781`, the `object`
`E0401`, and `::class` with `E0702`; examples 04 and 05 are the chapter's two error blocks.
`core-enums`' `details.md` has `E0401` for a number or another enum, `E0405` for a missing case
(an enum of your own gives `E0309`), the `as` message and `as ?`, `E0466`, `use Core\Arr;` and
`E0212`, an own enum of the same name, and JSON and `dump`; example 04 is a number in place of a case.

`60-iteration`'s first two features are done, each fact run on the binary, and no text disagreed
with it; the chapter did not change. `what-foreach-walks`' `details.md` has the key always being a
`string` (`E0723` for `int $k`), `E0401` for a written type, each `E0443` subject (a `?array<int>`
too), and a second loop over one `Iterator` going on from where the first stopped; example 04 shows
that. `the-two-interfaces`' has `E0442`, no type variable of your own, `E0449`, a class implementing
both, the three types not converting into each other (`E0401`, an `Iterator<int>` is not an
`Iterable<int>`), and the `current()` `LogicError` message; example 04 is the chapter's `E0444`
block. Probing found that `current(): string` on an `Iterator<int>` class compiles and a loop reads
the string's address as an `int`, now named in the override-signature gap record.

`60-iteration` is done: `generators`, `core-collections-are-iterable`,
`materialising-a-sequence-core-arr-from` and `what-does-not-exist` closed it, each fact run on the
binary. The chapter did not change, so `lang:iteration`'s perf stays current. One claim was wrong:
`what-does-not-exist`'s `about.md` said each error names what to write instead, but `E0303` for
`Countable` names nothing, and `E0320`'s help names no `Core\Arr` method; the sentence is gone.
`generators`' `details.md` has `E0446` (and `E0303` for `Generator`), `E0739` with no `yield`,
`E0445`, each `E0448` form, `E0401`, `E0447`, `E0492`, arguments copied at the call, `advance()`
after the end, `E0444`, `E0858` and the `clone` message, when `finally` runs, a `catch` in the body,
and `E0443` for a generator in `mixed`; examples 04 to 08 are the chapter's five blocks no example
showed (two of them compile errors). `core-collections-are-iterable`'s has the map being an
`Iterable<K>`, `E0401` and `E0444` for a map's value, a heap with a compare function, and a closed
channel's second loop. `materialising-a-sequence-core-arr-from`'s has the result type, an array
literal in the call giving `array<mixed>` (so `array<int> $b = Core\Arr::from([1, 2, 3]);` is
`E0401`), keys dropped, `E0401` for `mixed`, `limit`'s `uint` type, `limit: 0` running nothing,
`E0454`, and a limited generator continuing where it stopped. `what-does-not-exist`'s has `E0303`,
`E0320`, `E0482` and `E0443`; examples 04 and 05 are the chapter's two error blocks. Probing found
three generator bugs, owned by `release-hardening`. A `yield` of a literal into an `Iterator<?int>`
or `Iterator<int|string>` panics the compiler (`data/gaps/nvs-ir/a-generator-of-a-nullable-or.json`).
A `yield` in a `finally` stops at `E0901` (`data/gaps/nvs-ir/a-yield-inside-a-finally.json`). **A
generator that threw runs its body again from its last `yield` on the next `advance()`, and a
`finally` the throw left runs a second time when it is freed**
(`data/gaps/nvs-ir/a-generator-that-throws-is-not.json`). `looper verify` waited an hour for a
heavy slot another session's `--record-perf` held, and was stopped before it ran; the slice was
checked with `bun nv verify --doc`, `bun nv rules --check`, `bun nv owners`, `--comments` and each
feature's `--run --only`, all green.

`70-errors`' first four features are done, each fact run on the binary. One claim was wrong: the
chapter's property table said a throw at a file's top level has an empty `backtrace`, but it has one
`<script>() at file:line` frame; the table now says so, and that an object never thrown has an empty
one (`70-errors.md:87`). `the-throwable-tree`'s `details.md` has `E0303` for `Exception` and `Error`,
`new Throwable`, `ArithmeticError`'s messages and parent, the classes a user class may extend, and
`Core\Test\Failure` and `Core\Script::finish()`; example 04 is the chapter's `Exception` block.
`properties-not-accessors`' has each `get…()` and `code` (`E0405`), `E0782`, `location` before and
after a second throw, the top-level frame, `E0459` on `previous`, `issues`, and `dump`/`echo`/JSON of
an error; example 04 is the chapter's `getMessage` block. Its `about.md` said an error "knows", and
example 03 "asked of it", now rewritten. `constructing-and-subclassing`'s has `E0401` for an `int` or
`tainted` message and a wrong `previous`, `E0454`, named arguments, `E0410` and `E0402`. `throw`'s
has `E0780` (an interface-typed variable too), a `null` `?LogicError` or a non-`Throwable` `mixed`
throwing `LogicError` "Can only throw objects", where `throw` stands as a value, `E0401` for
`int $x = throw …`, and `E0320`; its `about.md` said "no answer for", now rewritten. Probing found two
gaps, owned by `release-hardening`: `implements Throwable` (or any class of the tree) compiles and
acts as `extends` (`data/gaps/nvs-types/implements-with-a-class-of.json`), and a `catch` naming a
non-throwable class compiles and never runs, while `catch (int $e)` stops at `E0901`
(`data/gaps/nvs-types/a-catch-clause-that-names-a.json`). `llms-full.txt` is rendered again. The
sweep's red jobs are run 39173391347 on `main` at be3e8a605, not this branch. The first
`looper verify` failed one `nvs-cli` agent test on Windows error 1455 (the paging file is too small)
while another session's `--record-perf` ran; the second run was green.

The next four `70-errors` features are done, each fact run on the binary. One claim was wrong in two
texts: the chapter and `lang:statements/try-catch-finally`'s `about.md` still said two `catch`
clauses in one function need two names, which the `20-types` slice had fixed only in `details.md`;
both now say clauses of different classes do (`70-errors.md:220`). `lang:errors/try-catch-finally`
keeps only what its chapter adds, and the shared `catch` facts stay in the statements feature's
`details.md`: its `details.md` has `E0250`, `E0251` (`break 2` too), a loop or a `try` inside
`finally`, the returned value computed before `finally`, `finally` running before an uncaught throw
ends the program, and a throw from `finally` replacing the error in flight. Examples 04 and 05 are
the chapter's `Loader` block and an `E0250` block. `rethrowing`'s `details.md` has a saved error
thrown again, `==` with it, the class kept through `catch (Throwable)`, `backtrace` keeping the
first thrower with the rethrow line for the rethrowing function, and `finally`. `an-uncaught-throw`'s
has the JSON and text report shapes (`[log] format`), the report having no `previous`, and
`finally`; example 04 is `finally` before the end (`// proof: exit 1`). `recursion-depth`'s has the
message, frame size changing the depth, a `catch` at any depth (example 04), `finally`, a
`backtrace` of thousands of entries and `new RecursionError`. Probing found two gaps, owned by
`release-hardening`. A throw from `finally` discards the error in flight with `previous` `null`,
which `rule:statements/no-return-leaves-a-finally` calls impossible
(`data/gaps/nvs-syntax/a-throw-inside-a-finally.json`). `if ($n == 100) { $n + 1 }` on an `int` is
`E0401`, since the comparison narrows `$n` to `100` and arithmetic on that is `mixed`
(`data/gaps/nvs-types/an-int-compared-with-a-literal.json`). `llms-full.txt` is rendered again.

`70-errors` is done: its last four features closed it, each fact run on the binary, and
`lang:errors`' perf is recorded after the chapter's last edit. Two claims were wrong. The chapter,
`inspecting-a-value`'s `about.md` and the syntax page `syntax/errors/assert-and-dump.mdx` said
`Core\Debug::render` returns text or a string; it returns a `Core\Cli\Text`, and `->text()` gives the
`string` (`70-errors.md:523`). The chapter said a path that does not exist is refused, but a missing
file inside a granted root throws `IOError` with the OS message; only one outside the roots is a
denial (`70-errors.md:471`). `fatal-errors-limits-no-catch-sees`' `details.md` has the `FATAL:` line,
the 1 megabyte reserve of a `16M` limit, the four `limit` values (each probed), the handler with no
parameter and `E0401`, a throw from the handler being dropped, the handler's `echo` after
`max_output`, and the two channel fatals, which do not run the handler. `capability-denials-are-catchable`'s
has the two-line message, `Core\IO::exists` needing `fs.read`, `..` and the missing file, and
`script.spawn`; example 01 now prints the `help:` line, as the chapter's block does. `assertion-failures`'
has `is`, `new Core\Test\Failure` and the uncaught report. `inspecting-a-value`'s has each type's
rendering (an enum case is its number), the 100-entry, 1024-byte and eight-level caps, `[cycle -> #1]`,
`dump()` with no value, `Core\Out::capture`, `E0724` and the request log. No chapter block needed a new
example. A run-time path given to `Core\IO::read` must be absolute; only a literal starts at the
file's folder, which a probe meets first. Recording the perf failed `lang:errors/throw`'s bench: it
declared `calls 2`, but since `ece7b500f` (merged from `main`) a `Core` call is counted too, so the
bench now declares `calls 3` (`benches/members/lang/errors/throw.nvs:6`). `main` still says 2. Any
other bench that calls a `Core` method fails the same way when its chapter is next recorded.

`80-concurrency`'s first four features are done, each fact run on the binary. One claim was wrong in
three texts: the chapter, `core-task-all-a-fixed-set-of-tasks`' `about.md` and the syntax page
`syntax/concurrency/tasks.mdx` said a field function with no return type gives `mixed`. A method
cannot omit it (`E0823`), a block `fn` cannot (`E0450`), and an expression `fn` gets its expression's
type; only a plain `callable` variable gives `mixed`. All three now say so (`80-concurrency.md:45`).
`no-colouring-i-o-just-waits`' `details.md` has `async`, `await` and `Fiber`, a task running until
it waits, a nested group, each task's `echo` printed in written order after the group (a stopped or
throwing task's too), and static properties. `core-task-all`'s has the field types, `tainted` after
`as string`, a `void` field, a missing field's `RuntimeError`, `{}` and `E0401`; example 04 is the
chapter's array block (a compile error). `core-task-map`'s has the parameter forms (the key is
always a `string`), the result type, a `callable(int): int` variable working where a plain
`callable` is `E0401`, and an `Iterator` or `mixed` subject being `E0401`. `limit-and-deadline`'s
has `E0453`, `E0454`, `limit`'s `uint` type, `deadline`'s `Duration` type, both messages,
`TimeoutError` being a `RuntimeError`, `deadline: 0ms`, a stopped task's `finally` not running, and
nested deadlines. `core-task-map`'s `about.md` said "holds", now rewritten. Probing found a gap,
owned by `release-hardening`: a `Core\Task::all` field function with a parameter compiles and the
call ends with an internal error (`data/gaps/nvs-types/a-task-group-function-that-takes.json`).
`llms-full.txt` is rendered again.

The next three `80-concurrency` features are done, each fact run on the binary. Three claims were
wrong. The syntax page `syntax/concurrency/limits-and-errors.mdx` said the other tasks are stopped
before they can throw; a task that throws before it waits is not, and its error goes to standard
error as `uncaught in a cancelled sibling: …`, by design (`crates/nvs-host/src/group.rs:481`). The
chapter and `syntax/concurrency/isolates.mdx` said `output: 'inherit'` interleaves the child's lines
with the parent's; it appends them at `await`, as `rule:security/isolate-output-is-captured` says,
and a handle never awaited prints nothing (`80-concurrency.md:241`). The chapter's result table said
`output` is a `string`; it is `Core\Html\Markup|Core\Cli\Text`, as the syntax page said
(`80-concurrency.md:261`). `a-child-s-throw`'s `details.md` has that, a stopped task's `finally`
not running, the same object and class reaching the `catch`, the one-entry `backtrace`, a nested
group, and a deadline winning; example 04 is two tasks throwing before they wait. The channel's
`details.md` has the `uint` capacity and the `0` fatal message, `E0442`, `E0401`, `var`, `E0444`, no
`receive` (`E0405`), the closed-`send` fatal that no `finally` sees, waiting forever with no reader,
a second loop, shared objects and a channel parameter; no new example, since 01 and 02 show the
chapter's block. The isolates `details.md` has the handle (`E0405`, `E0401` for `await`), `E0802`,
`E0101` with the five options, `E0818` for `on:` (a variable too), a computed `output:` and its
`LogicError`, the output type, `inherit`, an unawaited handle, `value` being `mixed` and `tainted`,
`exit` giving `ok` true, a returned object failing with the class `Error`, `args:` copied at the
spawn, a method entry's named `args:` (`E0402`, `E0486`, the run-time `LogicError`s), the
relative-path message, `limits:`, `grants: []`, and nested spawns. Examples 04 and 05 are the
chapter's `inherit` and second-`await` block, and its `with(args:…)` skip block made runnable, with
`jobs/say-hello.nvs` and `jobs/add-up.nvs`. Probing found a safety bug: **an instance method spawned
as `Class::method(...)` compiles, and `$this->n` reads `1` where the default is `7`**
(`data/gaps/nvs-types/an-instance-method-spawned-as-an.json`, owned by `release-hardening`). A
`send` on a full channel or a `foreach` on an open empty one with no other task waits forever, so a
probe of it needs a timeout. `llms-full.txt` is rendered again.

`80-concurrency` is done: `a-child-shares-nothing`, `a-child-s-failure-is-a-value` and
`what-does-not-exist` closed it, each fact run on the binary, and `lang:concurrency`' perf is
recorded. The chapter and the syntax page did not change. One claim was wrong:
`what-does-not-exist`'s `about.md` said each error names what to write instead, but `new Thread()`
and `new Fiber()` are a plain `E0303`, and `spawn worker` a general syntax error; the clause is gone.
`a-child-shares-nothing`'s `details.md` has an array static, `on: "here"`, a child started from a
task, and a child using a parent's variable (`E0301`, so the spawn throws). No new example: 01
shows the chapter's block. The failure feature's `details.md` has the `error` type (`E0459`
without `?->`), its two fields and the missing-field `RuntimeError`, the child's own class name,
`RecursionError` and `ArithmeticError`, a `try` around `await` catching nothing, `inherit`,
`exit(3)` with no status in the result, the one message for a missing file and a compile error, and
the denial's two lines. Its example 04 is the chapter's no-grant block, with its own `[[app]]`
entry in `nvs.toml` that sets `spawn = false`. `what-does-not-exist`'s `details.md` has `Thread`,
`Promise`, `parallel\Runtime`, `E0320` for `pcntl_fork` and `curl_multi_init`, `spawn worker`,
`E0204`, `E0211` and `E0209`; example 04 is the chapter's `Fiber` block. Probing found a gap, owned
by `release-hardening`: **a `throw` of an anonymous object compiles, and no `catch` sees it**, not
even `catch (Throwable)`, so `throw $r->error` in a parent ends the program
(`data/gaps/nvs-types/a-throw-of-an-anonymous-object.json`). The sweep's red jobs are run
39173391347 on `main` at be3e8a605, not this branch.

`90-attributes`' first three features are done, each fact run on the binary, and no text disagreed
with it; the chapter and the syntax page did not change. `an-attribute-is-an-anonymous-object`'s
`details.md` has `E0303` and `E0726` for the name, `E0401` with the missing field (also for `#[Name]`
with no list), no positional form, `E0494`, the allowed values, `E0725` for a variable, a call,
`new`, `1 + 2` and `{$x}`, `E0727`, the enum case and hook sites, `E0101` before a statement, and
repeats kept in order; example 04 is the chapter's `E0725` block. `reading-attributes-back`'s has
`E0728` (example 04), `E0729`, `E0442`, `E0730`, `E0471`, `E0798`, the promoted parameter, no
inheritance (a subclass and an inherited method find nothing), an interface method, `all<{}>`
counting `#[Core\Json\Derive]` too, constants folded where written (`Shop\K`), `E0792` with `E0731`,
and each call giving a copy. `the-names-the-compiler-acts-on`'s has `E0454`, `E0401`, `E0304`,
`E0767`, `E0726` for `#[Core\Nope]`, `E0405` for `Core\Json::Derive`, `E0212`, the bare form being
metadata, reading a `Core\Route` with a shape, and `E0789`; example 04 is the chapter's `colour`
block. Probing found three gaps, owned by `release-hardening`. `E::constructor(...)` on an enum
passes the checker: as a value it panics in lowering, and as a retrieval target it gives nothing, so
an attribute on an enum, an enum case or a class constant is never read
(`data/gaps/nvs-types/a-constructor-reference-to-an-enum.json`). An array literal in a payload is
`array<mixed>`, so an `array<int>` field is `E0401`
(`data/gaps/nvs-types/an-array-literal-in-an-attribute.json`). A `Core` attribute at a site where it
means nothing (`#[Core\Command]` above a class) compiles and does nothing, and
`#[Core\Json\Derive()]` with a list compiles and adds no codec
(`data/gaps/nvs-types/a-compiler-read-attribute-on-a.json`).

### Next

`90-attributes` goes on from `core-json-derive-and-core-json-field` (`90-attributes.md:171`), one or two of its long features per session;
take each feature id from `bun nv proofs --json`. Work each feature the way `break-and-continue` was done: read
its chapter section, its `about.md` and its syntax page, probe each fact `about.md` lacks on the
binary, write `details.md`, and turn each chapter code block no example shows into an example
(`proof: compile-error` for an `nvs error` fence). Bless with `bun nv proofs --bless <file>...`, then
`bun nv proofs --run --only <id>` (one `--only` per run: a second replaces the first) and
`--comments <dir>`.
A probe that reads a file needs an `nvs.toml` granting `[capabilities.fs] read` beside it, and a
probe is a file under `.agent-tmp/` written with Write, since the guard refuses a redirect.
A probe catching two error types needs two variable names, or it stops at `E0406` before the line
you meant to test. A probe's method goes in a class: a top-level `function` stops at `E0215`.
An enum is `enum Suit { Hearts, Spades }`, with no `case`. An array with string keys is still
`array<int>`: `array<string, int>` is `E0101`. Many probes at once: a JSON map of name to
body, and a script under `.agent-tmp/` that writes and runs each; the guard refuses a shell loop of
`nvs run` over `docs/examples/`, so check a new example with `--bless`.
A probe with a limit or a grant runs with `nvs run --config <file>.toml`, which replaces the root
`nvs.toml`; an example needs its own `[[app]]` entry in the root `nvs.toml` instead (`nvs.toml:497`).
A spawning probe has an `nvs.toml` with `[capabilities.script] spawn = true` beside it, and a probe
runner gives each run a timeout, since a channel or a task with nothing to wait for never ends.
- [ ] `90-attributes` from `core-json-derive-and-core-json-field-a-class-with-a-json-codec`, then the route table, the OpenAPI document, the command table and the two `Core\Program` features; the JSON, route, API and command sections are long and may each take a session. Then `bun nv proofs --record-perf --group lang:attributes` if the chapter changed. `docs/reference/lang/90-attributes.md:171`
- [ ] `95-testing`, which closes stage 6. `docs/reference/lang/95-testing.md:8`
- [ ] Once a chapter's last edit lands, `bun nv proofs --record-perf --group lang:<chapter>`, and commit `data/perf/members.ndjson`. `tools/nv/proofs/roster.ts:28`

### Backlog

- `docs/design.md` is deleted once looper's managed block no longer links it; the v3 block still does. `docs/goals/docs-cleanup.md:163`
- The v3 block's § *looper only orchestrates* says most of what AGENTS.md § *Novis works without looper* says. That section is the user's and stays; the overlap is looper's template to trim. `AGENTS.md:37`
- `docs/agent/testing.md` is looper-managed (`looper lint` refuses a hand edit) and links a `§ *Names*` that `conventions.md` lacks; looper's template owns the fix. `docs/agent/testing.md:16`
- Two diagnostics disagree with the binary's own behaviour: `E0233`'s help for a nested `use` says it
  "applies to the whole file", but a `use` covers only the code below it, and `E0303` for a class no
  `autoload` root has names no root. Product text, so it needs its own owner. `crates/nvs-types/src/locals.rs:2023`
- `E0802`'s help for an anonymous function as a spawn entry says "In `spawn worker` it would capture
  the variables around it", but `spawn worker` does not exist. Product text, same owner question. `crates/nvs-types/src/expr/isolate.rs:540`
- `E0479`'s help for a write through `?->` suggests `if ($x !== null)`, an operator Novis does not
  have; `and-the-ternary`'s example 05 prints it. Product text, same owner question. `crates/nvs-types/src/expr/assign.rs:1115`
- Many compiler diagnostics print a `rule:` id or the word "refused" in their help text, such as `E0234`, `E0413`, `E0448` and `E0230`. That text is held to § *Text an end user reads* when it is changed; this goal changes no product text, so it needs its own owner. `crates/nvs-types/src/expr/members.rs:2345`
- About 4,300 bare `§ N` marks under `crates/` and `tests/conformance/` cite sections of deleted decision records, and `RECORD_CITE` cannot tell them from an RFC's. Rewriting them needs its own goal; no owner yet. `tools/nv/cmd/rules.ts:84`
- The guard messages and `peek.ts`/`splice.ts` comments cite "AGENTS.md rule 1" and "rule 3" by the block's numbers, which a template change can move; citing `commands.md` sections would not. `tools/nv/cmd/guard.ts:259`
- `lang:statements/try-catch-finally` and `lang:errors/try-catch-finally` are one syntax and two features. Their `details.md` files split the facts, but their `about.md` files and examples 01 to 03 still say much the same; stage 7, which shows both on one syntax page, decides whether one `about.md` goes. `docs/examples/lang/errors/try-catch-finally/about.md:1`
- Stages 7 and 8 render the syntax pages and the chapters from each feature's `about.md` and `details.md`. `docs/goals/docs-cleanup.md:257`
