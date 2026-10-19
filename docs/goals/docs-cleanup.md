---
looper: 1
title: "docs/ holds only what the project's own work needs, and says each thing once"
summary: "docs/plan, docs/spec, docs/perf, docs/design.md, docs/future-ideas.md and docs/implementation-plan.md are gone with the tooling that read them, every public page is on the website alone, AGENTS.md is one document, docs/agent says each thing once, each language feature's about.md is the one text for its syntax page and its chapter section, and the checks below pass."
context: [AGENTS.md, docs/agent/doc-cleanup.md, website/README.md, tools/nv/cmd/rules.ts]
stages:
  - id: 1
    title: the plan and the design files are gone
    status: done
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
    status: done
    context: [docs/reference/lang/40-statements.md, docs/examples/lang/statements/break-and-continue/about.md, website/src/content/docs/syntax/control-flow/break-continue-return.mdx, tools/nv/proofs/roster.ts]
  - id: 7
    title: "the syntax pages show each feature's about.md, details and examples"
    status: done
    context: [website/src/lib/feature.ts, tools/nv/cmd/site.ts, website/src/content/docs/syntax/control-flow/break-continue-return.mdx]
  - id: 8
    title: the chapters under docs/reference/lang are generated from about.md and details.md
    status: done
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
- [x] `docs/plan/`, `docs/implementation-plan.md`, `docs/design.md`, `docs/future-ideas.md` and `data/plan/status.json` are deleted. `nv plan` and its tests, `schema/milestone.ts` and its foreign keys, `owners.ts`'s milestone kind, `pack.ts`'s milestones, `wrap.ts`'s `PLAN_KINDS`, CI's `plan --check` and the settings entry go with them. `tools/nv/cmd/plan.ts:200`, `tools/nv/schema/milestone.ts:1`, `tools/nv/schema/gap.ts:18`, `tools/nv/schema/rule.ts:22`, `tools/nv/lib/wrap.ts:19`, `.github/workflows/ci.yml:978`, `.claude/settings.json:60`
  - [x] the tooling: `nv plan`, `wrap.ts`, both schemas, `data/plan/`, the milestone kind and fields, CI and settings
  - [x] `docs/plan/`, `docs/implementation-plan.md`, `docs/future-ideas.md` and their rows in CONTRIBUTING.md's layout listing
  - [x] `docs/design.md`: `looper.toml` sets `[project] design = ""`, so looper's managed block links no architecture doc
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
- [x] The other ten chapters, one per slice: `10-programs`, `20-types`, `30-expressions`, `50-classes`, `55-enums`, `60-iteration`, `70-errors`, `80-concurrency`, `90-attributes`, `95-testing`. `docs/reference/lang/10-programs.md:1`
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
  - [x] `90-attributes` and `95-testing`
    - [x] `90-attributes`, then `lang:attributes`' perf recorded after the chapter's last edit
    - [x] `95-testing`, then `lang:testing`' perf recorded after the chapter's last edit

### 7. the syntax pages show each feature's about.md, details and examples

**Does:** every syntax page shows the `about.md`, details and examples of each feature it covers, and
repeats none of their text.

- [x] `feature.ts` reads `lang` like the other areas, and the component the `Core` pages use renders a `details.md`. `website/src/lib/feature.ts:33`
  - [x] `feature.ts` globs `lang`, and `detailsFor` reads a `details.md`
  - [x] `FeatureSection.astro` renders the `details.md` under an `h3` after the examples
  - [x] `bun nv site --check syntax` accepts a `<FeatureSection>` where it needed a `<Snippet>`, with a case in `site.test.ts`
- [x] Each syntax page renders its covered features, keeps its own frame, *Use this when* and *Do and don't*, and loses its snippets for those features. `website/src/content/docs/syntax/control-flow/break-continue-return.mdx:1`
  - [x] `control-flow/break-continue-return.mdx`, stamped
  - [x] the rest of `control-flow/`
  - [x] `errors/`, `functions/`, `operators/` and `basics/`
  - [x] `values/`, `programs/` and `classes/`
  - [x] `concurrency/` and `attributes/`
  - [x] `testing/`
  - [x] a page for the three `lang:types` features no page covers
- [x] The site lock hashes a `lang` feature's `about.md`, `details.md` and examples. `tools/nv/cmd/site.ts:306`
  - [x] `featureHash` already hashes every file in the feature's directory; a case in `site.test.ts` pins a changed `details.md` and example

### 8. the chapters under docs/reference/lang are generated from about.md and details.md

**Does:** no chapter section is hand-written, and `nvs agent` prints the same text the website does.

- [x] Each chapter's front matter and feature order have a source, `docs/examples/lang/<chapter>/chapter.md`, and the roster reads its ids from it, with no id changed. `tools/nv/proofs/chapters.ts:1`
  - [x] the format and its parser, with cases in `tools/nv/test/chapters.test.ts`
  - [x] the eleven sources, seeded from the chapters' front matter, headings and markers
  - [x] `langFeatures` in `roster.ts` builds the entries from the sources and stops on a chapter that disagrees
  - [x] the examples README and the reference README name the file
- [x] A renderer writes each chapter from its features' `about.md` and `details.md` in roster order, and a `--check` refuses a hand edit. `tools/nv/renderers/lang-chapters.ts:1`
  - [x] `lang-chapters` in `RENDERERS`: the source's front matter, then per feature its texts and one `## Example NN` per example, with the fence its proof directive names, cases in `tools/nv/test/lang-chapters.test.ts`
  - [x] the examples that took a grant from the root `nvs.toml` carry their own, so a chapter example runs the way the examples tree runs it
  - [x] `bun nv reference` holds all 664 examples, and `render --check` is current
  - [x] the reference README, the examples README, `chapters.ts` and `reference.ts` say the chapters are generated
- [x] `nvs agent primer` and `show`, and `/llms-full.txt`, read the generated chapters, and their tests pass unchanged in shape. `crates/nvs-cli/src/agent.rs:187`
  - [x] the five `agent.rs` tests that quoted the old prose read the generated text, and every syntactic form is still reached through a heading
  - [x] `site.ts`'s `card` is empty for a `lang` feature, and the 63 pages that went stale through it are stamped
  - [x] `website/public/llms-full.txt` rendered again
  - [x] every `lang` group's perf recorded after the render
  - [x] the full `looper verify` green on the commit

## Handoff

### State

Every stage is done, and the goal is reached. `docs/design.md` is deleted: `looper.toml` sets
`[project] design = ""`, which drops the row looper's managed block linked it from, and
`DELETED_DOC_CITE` refuses a new citation of it (`tools/nv/cmd/rules.ts:107`). The goal-end gates
pass on this branch: `bun nv verify --doc` finds every link resolving, `bun nv owners --closes
docs-cleanup` finds no gap this goal owns, `bun nv rules --check` is clean and `looper verify` is green.

The decisions only this file held have homes: the website as the home of everything public is in
`docs/agent/doc-style.md`, and `details.md` is held to AGENTS.md § *Text an end user reads*. Every
backlog item is a gap record under `data/gaps/` or side goal `section-marks`.

### Next

- [ ] Write `DONE docs-cleanup is reached` to the status file, with no other change: looper runs the acceptance checks, retires the goal and lands the branch. `docs/goals/docs-cleanup.md:1`

### Backlog

- None. Each item moved to a gap record or to side goal `section-marks`. `docs/goals/section-marks.md:1`
