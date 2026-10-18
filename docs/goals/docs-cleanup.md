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
  - [ ] `20-types`
    - [x] `every-binding-has-a-type`, `numbers-bool-int-uint-float-decimal`, `text-string-and-bytes` and `array-t`
    - [x] `callable-classes-object-shapes`, `mixed`, `nullable-union-single-value-and-enum-case-types` and `void-never-self-static`
    - [ ] `type-aliases`, `values-written-in-the-code`, `widening-without-as` and `the-conversion-operator-as`
      - [x] `type-aliases`
      - [ ] `values-written-in-the-code`
      - [ ] `widening-without-as`
      - [ ] `the-conversion-operator-as`
    - [ ] `property-keys`, `property-paths`, `strings-with-holes` and `narrowing`
    - [ ] `truthiness`, `parameters`, `properties-and-constants`, `qualifiers-tainted-and-secret` and `what-does-not-exist`
  - [ ] `30-expressions`, `50-classes` and `55-enums`
  - [ ] `60-iteration`, `70-errors` and `80-concurrency`
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

`looper lint` failed on a stale managed block, so `looper sync` brought AGENTS.md's `agents` block to
v3. It adds § *looper only orchestrates* and still links `docs/design.md`.

The sweep's red items are not this branch's: run 39108526796 ran on `main` at 1a78b49ef, as earlier
sessions found, and the red bench `benches/members/core/Ldap-Connection/delete.nvs` belongs to the
`ldap` work on `main`. `bun nv site --check` also fails on pages this goal has not touched.

### Next

`20-types` from `the-conversion-operator-as` (10 features left, and it is the longest section at 176
lines, so take it alone), one feature per slice, the way `break-and-continue` was done: read
its chapter section, its `about.md` and its syntax page, probe each fact `about.md` lacks on the
binary, write `details.md`, and turn each chapter code block no example shows into an example
(`proof: compile-error` for an `nvs error` fence). Bless with `bun nv proofs --bless <file>...`, then
`bun nv proofs --run --only <id>` (one `--only` per run: a second replaces the first) and
`--comments <dir>`.
A probe that reads a file needs an `nvs.toml` granting `[capabilities.fs] read` beside it, and a
probe is a file under `.agent-tmp/` written with Write, since the guard refuses a redirect.
A probe catching two error types needs two variable names, or it stops at `E0406` before the line
you meant to test. A probe's method goes in a class: a top-level `function` stops at `E0215`.
- [ ] `20-types` from `the-conversion-operator-as`, in chapter order. `docs/reference/lang/20-types.md:632`
- [ ] Once the chapter's last edit lands, `bun nv proofs --record-perf --group lang:types`, and commit `data/perf/members.ndjson`. `tools/nv/proofs/roster.ts:28`

### Backlog

- `docs/design.md` is deleted once looper's managed block no longer links it; the v3 block still does. `docs/goals/docs-cleanup.md:163`
- The v3 block's § *looper only orchestrates* says most of what AGENTS.md § *Novis works without looper* says. That section is the user's and stays; the overlap is looper's template to trim. `AGENTS.md:37`
- `docs/agent/testing.md` is looper-managed (`looper lint` refuses a hand edit) and links a `§ *Names*` that `conventions.md` lacks; looper's template owns the fix. `docs/agent/testing.md:16`
- Two diagnostics disagree with the binary's own behaviour: `E0233`'s help for a nested `use` says it
  "applies to the whole file", but a `use` covers only the code below it, and `E0303` for a class no
  `autoload` root has names no root. Product text, so it needs its own owner. `crates/nvs-types/src/locals.rs:2023`
- Many compiler diagnostics print a `rule:` id or the word "refused" in their help text, such as `E0234`, `E0413`, `E0448` and `E0230`. That text is held to § *Text an end user reads* when it is changed; this goal changes no product text, so it needs its own owner. `crates/nvs-types/src/expr/members.rs:2345`
- About 4,300 bare `§ N` marks under `crates/` and `tests/conformance/` cite sections of deleted decision records, and `RECORD_CITE` cannot tell them from an RFC's. Rewriting them needs its own goal; no owner yet. `tools/nv/cmd/rules.ts:84`
- The guard messages and `peek.ts`/`splice.ts` comments cite "AGENTS.md rule 1" and "rule 3" by the block's numbers, which a template change can move; citing `commands.md` sections would not. `tools/nv/cmd/guard.ts:259`
- `lang:statements/try-catch-finally` and `lang:errors/try-catch-finally` are one syntax and two features, each with an `about.md` and three examples saying much the same. Stage 6's `70-errors` slice decides whether the errors one keeps only what its chapter adds. `docs/examples/lang/errors/try-catch-finally/about.md:1`
- Stages 7 and 8 render the syntax pages and the chapters from each feature's `about.md` and `details.md`. `docs/goals/docs-cleanup.md:257`
