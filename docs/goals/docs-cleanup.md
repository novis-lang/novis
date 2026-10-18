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
    title: the Core spec is gone, and the website's Core pages are built from the registry
    status: done
    context: [docs/spec/01-core-library.md, tools/nv/renderers/website-core.ts, tools/nv/renderers/spec-tables.ts, crates/nvs-stdlib/tests/spec_registry_coverage.rs]
  - id: 3
    title: docs/perf is gone, its ledgers are data and its findings have homes
    status: open
    context: [tools/nv/proofs/ledger.ts, tools/nv/proofs/perf.ts, tools/nv/cmd/scaling.ts, docs/perf/userland-gap.md, docs/goals/fewer-allocations.md]
  - id: 4
    title: every public page is on the website alone
    status: open
    context: [docs/docker.md, docs/why-tainted-and-secret.md, docs/release.md, tools/nv/cmd/reference.ts, website/src/content/docs/guides/production/docker.mdx]
  - id: 5
    title: AGENTS.md is one document, and docs/agent says each thing once
    status: open
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
    title: "the chapters under docs/reference/lang are generated from about.md and details.md"
    status: open
    context: [tools/nv/proofs/roster.ts, crates/nvs-cli/src/agent.rs, tools/nv/cmd/reference.ts]
checks:
  - id: rules
    stage: 1
    name: the rulebook is clean, and no file cites a deleted doc
    run: [bun, nv, rules, --check]
  - id: doc-links
    stage: 4
    name: every doc link and anchor resolves
    run: [bun, nv, verify, --doc]
  - id: lint
    stage: 5
    name: goals, chain, README and playbook lint clean
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
- **`rule:testing/perf-history-file` and `rule:testing/perf-secondary-figures` are retired**, with
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

- [ ] `members.ndjson`, `members.md` and `bench-budget.json` move to `data/perf/`, with every path constant, the `ci-changes.ts` lane, `brief.ts`'s homes and the rule records' `guardedBy`. `tools/nv/proofs/ledger.ts:11`, `tools/nv/proofs/perf.ts:36`, `tools/nv/cmd/scaling.ts:156`, `tools/nv/cmd/ci-changes.ts:91`, `data/rules/testing/member-perf-ledger.json:10`
- [ ] `history.ndjson` is deleted, and `rule:testing/perf-history-file` and `rule:testing/perf-secondary-figures` are retired. `data/rules/testing/perf-history-file.json:10`
- [ ] The three open findings with no record become gap records: an unread POST body of 16 KB or more can lose the response, a head of 100 header lines is answered 431, and `nvs lsp` grows near-quadratically. `docs/perf/performance-pass.md:1`
- [ ] `performance-review.md`'s *Bounded* costs go into each module's `//!` cost section, and `scaling --areas --reviewed` is retired with it. `tools/nv/cmd/scaling.ts:1046`
- [ ] `userland-gap.md` and `brainsort-evaluation.md` give their still-true lines to `crates/nvs-runtime/src/callable.rs`, `crates/nvs-stdlib/src/sort.rs` and `crates/nvs-stdlib/src/arr.rs`. Goal `fewer-allocations` states its own design where it cites § F, § I and the `Markup` decision, and its stage 9 loses the items this stage does. `docs/goals/fewer-allocations.md:91`, `docs/goals/fewer-allocations.md:167`
- [ ] Every citation of `docs/perf/` is rewritten, including `perf_guards.rs`'s assertion message, and `docs/perf/` joins `RECORD_CITE`. `benches/abi-probe/tests/perf_guards.rs:2814`

### 4. every public page is on the website alone

**Does:** a user finds every public fact on the website, and `docs/` holds no copy of one.

- [ ] `docs/docker.md`'s public parts join `guides/production/docker.mdx`: the image and its tags, the variants, Compose, CLI use, stopping, health probes and verifying. The stale "one process on one core" section is dropped. Building locally and *Why not Alpine* go to `docker/README.md`. `release.yml`'s documentation label, the Dockerfile, `smoke.sh` and `.gitignore` are repointed. `.github/workflows/release.yml:588`, `docker/Dockerfile:15`
- [ ] `docs/why-tainted-and-secret.md`'s three readers, its comparison table and its five points become a website page under `in-depth/`. *What not to claim yet* goes to `rule:programs/three-claims`. `README.md:17` is repointed. `website/src/content/docs/in-depth/concepts/tainted-input.mdx:1`
- [ ] `docs/release.md` keeps the procedure. *Verifying a published binary* goes to the install page, the user-facing half of the version scheme goes to the website, and its wrong counts are fixed. `docs/release.md:180`, `tools/nv/cmd/release.ts:88`
- [ ] `bun nv reference` writes `/llms-full.txt` into the website build, and `docs/novis.md`, its CI `--check` and verify's staleness step are deleted. `tools/nv/cmd/reference.ts:786`, `.github/workflows/ci.yml:892`, `tools/nv/cmd/verify.ts:1047`
- [ ] `docs/reference/tools/30-php-differences.md` gets a website page, and `README.md:44` is repointed. `docs/reference/findings.md`'s open items become gap records, and the file is deleted with its citations. `tests/conformance/core/arr-sort-orders-decimals.nvst:9`

### 5. AGENTS.md is one document, and docs/agent says each thing once

**Does:** AGENTS.md says each thing once, every file under `docs/agent/` is current and unique, and
the playbook holds only traps that can still happen.

- [ ] AGENTS.md outside the markers holds only what the managed block lacks, and no rule number, step count or budget contradicts it. The text names the mid-work check, `bun nv verify --fast -p <crate>` after each slice, and the shell shapes the guard refuses, each with the call to make instead. `AGENTS.md:1`, `AGENTS.md:392`
- [ ] `commands.md` and `conventions.md` lose every section another home holds, every measured figure and every story, and the dead `§ Names` anchor and section references are fixed. `docs/agent/commands.md:24`, `docs/agent/conventions.md:494`, `docs/agent/testing.md:16`
- [ ] `optimization-prompt.md`, `carried-refusals.md` (with its readers in `holes.ts`, `owners.ts` and `refusals.rs`) and `guard-name-debt.md` are deleted; its three causes move to `commands.md` or `testing.md`. `coordinator.md` and `loop-authoring.md` lose their history and stale numbers. `docs/agent/coordinator.md:19`, `tools/nv/cmd/owners.ts:38`
- [ ] The playbook loses every bullet about a removed tool, every story, every bullet that contradicts a rule (such as the `git stash` one) and every duplicate, and `[orient] traps = 8`. `docs/agent/playbook/running-things/a-before-after-measurement-is-worth-a-git-stash-and-the-base.md:1`, `looper.toml:8`
- [ ] `how-novis-is-written.mdx`'s count of notes comes from `numbers.json` or goes. `website/src/content/docs/in-depth/how-novis-is-written.mdx:62`

### 6. each language feature's facts are in its about.md, its details.md and its examples, once

**Does:** for every language feature, every fact its chapter section, its syntax page and its
`about.md` state is in exactly one of `about.md`, `details.md` or an example, and the binary agrees
with each.

- [ ] The proofs tooling knows `details.md`: optional, under the feature's examples folder, and held to the comment bounds. Whether an example can show a compile error is checked, and added if it cannot. `tools/nv/proofs/roster.ts:27`
- [ ] `40-statements` is done first, as the pattern: each feature's facts sorted into its three homes, the chapter's unique code blocks made examples. `docs/reference/lang/40-statements.md:244`, `docs/examples/lang/statements/break-and-continue/about.md:1`
- [ ] The other ten chapters, one per slice: `10-programs`, `20-types`, `30-expressions`, `50-classes`, `55-enums`, `60-iteration`, `70-errors`, `80-concurrency`, `90-attributes`, `95-testing`. `docs/reference/lang/10-programs.md:1`

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

Stage 1 is done but for one file. The plan tooling, `docs/plan/`, `docs/implementation-plan.md` and
`docs/future-ideas.md` are gone, and every citation of them is rewritten. The thread-per-core
decision's home is now `crates/nvs-host/src/scheduler.rs`'s module doc with
`rule:concurrency/one-scheduler`; per-request isolation is `rule:security/arena-is-an-ownership-root`;
the unsafe policy is `tools/nv/cmd/lints.ts`'s header. `bun nv rules --check` fails on a new
citation of a deleted doc outside `docs/goals/` (`deletedDocFindings`, `tools/nv/cmd/rules.ts:113`).

**Blocked on looper, not on this repo:** `docs/design.md` stays as a five-line pointer. Looper's
managed block in AGENTS.md links it (`AGENTS.md:392`), and `looper lint` refuses a hand edit inside
the markers. It goes once looper's `agents` template stops linking it; then add `design\.md` back to
`DELETED_DOC_CITE` and tick stage 1's last sub-item. Stage 1 stays `open` until then.

Stage 2 is done. `website-core.ts` reads only `nvs meta --json` and the class introductions, the
spec's machinery and `docs/spec/` are deleted, and `bun nv rules --check` refuses a citation of the
spec by path, by relative link or as `spec § N` in either case (`DELETED_DOC_CITE`,
`tools/nv/cmd/rules.ts:107`). Three spec claims the audit found already contradicted (`Instant`'s
`<`, `Task::all`'s callable field, `decodeAs`'s inline shape) were not moved anywhere.

The bare `§ N` marks were classified by a script: about 4,300 lines in 631 files under `crates/` and
`tests/conformance/` name no document on the line or the one before. About 30 meant the spec, found by
a section number matching the spec's section of a class named beside it, and they are rewritten to the
class. Nearly all the rest are sections of decision records an earlier goal deleted, with the record's
name already stripped: `crates/nvs-stdlib/src/db/transaction.rs:202`'s `§ 7's ROLLBACK` came in with
ADR 0067 (`git log -S`). Those are not this goal's; see the backlog.

The sweep's release-build failure was the full disk: `E:` has 62 GB free now, and `cargo build
--release -p nvs-cli` passes. The nightly CI failures are on `main`'s commit `e110c9c`: `docs`
(`routing/a-gate-answers-around-every-handler` names no owner), `proofs` on three platforms (about
280 `perf: stale` members), the Windows warm-start budget and the db matrix. None is this goal's.

### Next

Stage 3 on `tools/nv/proofs/` and `tools/nv/cmd/`: the perf ledgers move to `data/perf/`, and the
history file and its two rules go.
- [ ] `members.ndjson`, `members.md` and `bench-budget.json` move to `data/perf/`, with every path constant, the `ci-changes.ts` lane, `brief.ts`'s homes and the rule records' `guardedBy`. `tools/nv/proofs/ledger.ts:11`, `tools/nv/proofs/perf.ts:36`, `tools/nv/cmd/scaling.ts:156`, `tools/nv/cmd/ci-changes.ts:91`, `data/rules/testing/member-perf-ledger.json:10`
- [ ] `history.ndjson` is deleted, and `rule:testing/perf-history-file` and `rule:testing/perf-secondary-figures` are retired. `data/rules/testing/perf-history-file.json:10`
- [ ] `tests/conformance/core/arr-every-callback-is-offered-value-and-key.nvst:12` cites `docs/perf/userland-gap.md`, which stage 3's last item rewrites with every other `docs/perf/` citation. `docs/goals/docs-cleanup.md:202`

### Backlog

- `docs/design.md` is deleted once looper's managed block no longer links it. `docs/goals/docs-cleanup.md:163`
- About 4,300 bare `§ N` marks under `crates/` and `tests/conformance/` cite sections of deleted decision records, and `RECORD_CITE` cannot tell them from an RFC's. Rewriting them to the `rule:` id each meant is several sessions and needs its own goal; no owner yet. `tools/nv/cmd/rules.ts:84`
- Stages 6 to 8 put each fact about a language feature in one place: its `about.md`, a new `details.md` or an example. The syntax pages and the chapters are built from them. `docs/goals/docs-cleanup.md:213`
