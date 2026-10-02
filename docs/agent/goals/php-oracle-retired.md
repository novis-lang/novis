---
milestone: post-parity
position: last
---
# Loop goal 187 — nothing in Novis is tested against PHP or translated from it, and PHP is named only where a page compares the two

PHP was the oracle that let the language be prototyped fast: a case ran under both engines, and a
difference was either a bug or a recorded divergence. That era is over. Novis has diverged a long way
from PHP, and every behaviour is now stated by its own rule and pinned by its own conformance case.
This goal removes every test and every comparison that runs PHP to judge Novis, and it moves PHP out
of the prose that does not compare anything.

**It also ends the idea of mechanical migration.** A program is not moved to Novis by translating it
line by line or name by name. Novis thinks differently and has capabilities PHP lacks, and a mechanical
port carries the PHP shape over and misses them. A program is moved by rethinking it — by hand, or by an
AI agent working from the binary's own `nvs agent` topics. So M11 and `nvs convert` leave the plan, and
every list that maps a PHP name or construct to a Novis one is deleted, with the features that read it.
What stays is **one concept page**: how Novis thinks differently, what it can do that PHP cannot, and how
to port a program by rethinking it.

```text
removed   tests/differential/ (281 cases), the --ORACLE-- and --ORACLE-DIVERGES-- sections, `nvs test --php`,
          bun nv try's PHP twin, the editor grammar's embedded PHP, the oracle playbook entries,
          the --INI-- section, M11 and every plan of `nvs convert` and a .phpt importer, the convert rules,
          docs/spec/02-php-migration.md and data/spec/php-migration.json, tools/data/php-builtins.txt and its
          generator, the `Replaces` column, `bun nv migration`, nvs_stdlib::php_names, the editor's PHP-name
          completion and `nvs.completion.phpNames`, E0320's PHP clause, the `divergesFromPhp` field,
          docs/divergences.md, the php-migration rule topic, the migration claims page
kept      the php-differences chapter, cut to one concept page, with its examples and attacks; the PHP
          benchmarks (benches/userland, --serve-vs-fpm, benches/proxied); the diagnostics for a PHP habit
          (E0229 `<?php`, W1004, the superglobals and PHP-constant cards)
deleted   priority 2 of the priority ordering, outright: four priorities remain
decided   the three gaps M11 was holding, as Novis's own behaviour
rewritten every citation of a priority by its number, the rules that make PHP normative, the language rules
          filed under php-migration/, about.md and registry text that opens with or ends on
          "This replaces PHP's `x`."
```

## Why here

**Last on the chain, by the user's decision of 2026-10-01**: once every planned goal is done the
language is feature complete, and this is the cleanup that closes the prototyping era. It sits behind
goal `coverage-depth` and carries `position: last` for that reason.

**Behind goal `goal-closeout` for a second reason.** Two walked goals, `tooling-overhaul` and
`core-io-file-and-1-more`, carry differential checks into today's floor. Goal `goal-closeout` ends the
carried floor and deletes the walked goals with their records, so when this goal is reached no goal
record names `tests/differential/` and nothing here edits a walked goal.

## Stage 0 — the catch-up

None. Every sentence this goal makes untrue is rewritten by the stage that makes it untrue.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. After goal `goal-closeout` that is nothing: the
suites, the `.nvst` trees and `nv verify` are the safety net. `tests/conformance/` is never weakened
here — no case is deleted from it, and no frozen output in it is edited except where Stage 6 says.

## Stage 2 — the record, and priority 2 is deleted

**Does:** Writes the decision record for the whole goal, deletes priority 2 from the ordering, and makes every citation name a priority instead of numbering it.

Two file sets, in this order. The ordering: `AGENTS.md`, `data/rules/programs/memory-priority.json`
and its fragment, `data/rules/php-migration/every-divergence-is-deliberate-and-listed.json` and its
fragment, `website/src/content/docs/docs/index.mdx`, `docs/agent/commands.md`,
`docs/agent/user-report.md`. Then the citations: the rule fragments and Rust comments listed below.

- **The decision record**, written first, for the whole goal. It `modifies` ADR 0004's priority list
  through `programs/memory-priority`, and `testing/nvst-is-separate` and
  `ide/case-files-have-their-own-grammar`. It records every rule this goal deletes or moves — the
  `tooling/convert-*` rules, the whole `php-migration/` topic, the PHP-name completion rules — in a
  section of its own, because `changes:` has no key for a removal (the shape commit 3759c8451 used for
  ADR 0192). It names the records whose decisions it ends: ADR 0089, behind every `tooling/convert-*`
  rule, and ADR 0111, behind the PHP-name completion layer. It states the user's calls and the tradeoffs from § *Standing decisions*, and the
  old-to-new mapping of the ordering, so a frozen record that says "priority 3" can still be read.
- **Priority 2 is deleted, by the user's call.** "Correctness of language semantics — PHP-compatible
  observable behaviour" leaves `AGENTS.md:51`, `docs/rules/programs/memory-priority.md:5` and
  `website/src/content/docs/docs/index.mdx:25`, and is not replaced. The ordering is four items:
  security and request isolation, latency and throughput on the request path, simplicity, memory
  footprint. What a program observably does is stated by the rules and pinned by the conformance
  cases, and the ordering does not repeat it. This also ends the contradiction with
  `rule:programs/no-compatibility-promise`, which forbids "PHP compatible" in any document.
- **A priority is cited by its name, never its number.** The numbers shift, so every current citation
  is rewritten to name the priority it means ("latency", "memory footprint"), and a name survives any
  later change to the list. Known sites: `AGENTS.md:65`, `docs/rules/programs/memory-priority.md:21`,
  `docs/rules/observability/an-exporter-brings-no-second-scheduler-and-no-second-client.md:20`,
  `docs/rules/packaging/the-notice-is-embedded-in-the-binary.md:12`,
  `docs/rules/packaging/the-windows-binary-says-what-it-is.md:33`,
  `docs/rules/routing/a-capture-narrows-to-a-closed-set.md:13`,
  `docs/rules/security/isolate-teardown-is-a-drain-then-a-sweep.md:11`, and the Rust comments that
  cite the ordering by number (`git grep -n -i -E "priority [1-5]\b" -- crates` lists the candidates;
  a scheduler or task priority is a different thing and stays). Frozen records keep their numbers.
- `docs/agent/commands.md:418` and `docs/agent/user-report.md:54` stop naming PHP behaviour as what a bug
  is measured against. A bug is Novis doing what a rule or the reference says it does not.
- The playbook bullet whose `[until:]` is keyed on the old `AGENTS.md` text
  (`data/playbook/tooling/a-next-group-items-rationale-about-php-behaviour-is-a.json`) retires with it.
  That is intended.
- **Pinned by** the Stage 2 checks.

## Stage 3 — every error message has a conformance case

**Does:** Moves every assertion only `tests/differential/` makes into `tests/conformance/`, so deleting the tree loses nothing.

One file set: `crates/nvs-stdlib/tests/conformance_coverage.rs`, `tools/nv/cmd/gaps.ts`, and new cases
under `tests/conformance/`.

- `crates/nvs-stdlib/tests/conformance_coverage.rs:@error_corpus` reads both trees, because some
  messages are asserted only in a differential case (`:280-282`). `bun nv gaps --errors` reads the same
  corpus on purpose (`tools/nv/cmd/gaps.ts:343-362`). Both become `tests/conformance/` alone in one
  slice.
- Every message that then goes unasserted gets a conformance case with an `--EXPECT--` or
  `--EXPECTF-ERROR--` written from the Novis output. A differential case whose `--ORACLE-DIVERGES--` pinned
  a behaviour no conformance case pins gets a conformance case too. **The output is copied from what
  Novis prints today**, never from the PHP twin.
- The rule fragments and decision records whose `guardedBy` or `validatedBy` names a differential case
  (`classes/clone-is-shallow`, `expressions/switch-match-equality`,
  `statements/static-is-a-member-modifier`, `data/decisions/0144.json`, `data/decisions/0211.json`)
  are pointed at the conformance case that now pins the same behaviour.
- **Pinned by** the Stage 3 check: the existing coverage test, plus a new named test that the corpus is
  the conformance tree alone.

## Stage 4 — the oracle is gone

**Does:** Deletes `tests/differential/`, the oracle sections, `nvs test --php`, the `--INI--` section and every reader of them.

Three file sets, in this order.

1. **The runner.** `crates/nvs-test/src/case.rs` (`:31-36`, `:342-343`, `:526-563`),
   `crates/nvs-test/src/run.rs` (`php_available` at `:183`, the skip at `:196`, the comparison at
   `:337-355`, `run_php` at `:444`), `crates/nvs-test/src/lib.rs:347-351`,
   `crates/nvs-cli/src/main.rs:463-465` and `:3198`. `--ORACLE--` and `--ORACLE-DIVERGES--` become
   unknown sections, refused with the error every unknown section gets. The `Oracle` enum, the `--php`
   flag and `php_available` are deleted. `crates/nvs-test/src/expect.rs:291`'s test is renamed for what
   the wildcards match, not for PHP. `crates/nvs-lsp/src/case.rs:639-641` keeps refusing the section in
   an `.lspt`, because every unknown section is refused there. `--INI--` is already one.
2. **The tree and the tools.** `git rm -r tests/differential`. Then `tools/nv/cmd/verify.ts:145` and
   `:904` (the step), `tools/nv/lib/chain.ts:119,147-150`, `tools/nv/cmd/gaps.ts` (`--differential`),
   `tools/nv/cmd/session.ts:223-227,1058`, `tools/nv/cmd/orient.ts:1451`, `tools/nv/cmd/proofs.ts:185`,
   `tools/nv/proofs/markers.ts:6,9`, `tools/nv/cmd/affected.ts:259`, and their tests
   (`tools/nv/test/chain.test.ts`, `verify.test.ts`, `select-checks.test.ts`). `bun nv try`
   (`tools/nv/cmd/try.ts`) keeps running Novis snippets several at a time and loses its PHP twin. Its
   module doc is rewritten whole.
3. **The editor and the prose.** `editors/vscode/syntaxes/nvst.tmLanguage.json:38,78` and the grammar
   tests and fixture that embed `source.php`. `.github/workflows/ci.yml:177-182`'s comment.
   `docs/setup.md` says PHP is needed only to run the PHP benchmarks.
   `docs/agent/conventions.md` § *A `.nvst` test case* and `docs/agent/commands.md` § `try` lose the
   oracle. `docs/reference/lang/95-testing.md:401` and `docs/reference/tools/10-cli.md:172,231` (with
   `data/reference/tools/10-cli.json`) lose `--ORACLE--` and `--php`. The playbook entries about oracles
   and divergence sections under `data/playbook/` are deleted. The source comments that cite a
   differential case (`crates/nvs-ir/src/lower/call.rs:821`, `crates/nvs-stdlib/src/arr.rs:3336,5756`,
   `regex.rs:1787,2139`, `uri.rs:248`, `str.rs:82`) cite the conformance case instead.

- **Pinned by** the Stage 4 checks.

## Stage 5 — no mechanical migration is planned

**Does:** Deletes milestone M11, `nvs convert` and the `.phpt` importer from the plan, the rules, the website and the README.

Two file sets, in this order.

1. **The plan.** `docs/plan/m11.md` and `data/plan/milestones/M11.json` are deleted. Its row leaves
   `docs/implementation-plan.md:111` and `README.md:64`, and the plan's `Open now` and `Blocking`
   fields stop naming it. The live milestone files that lean on it are rewritten whole where they do:
   `docs/plan/m4.md:33`, `docs/plan/m4s.md:16`, `docs/plan/m10.md`, `docs/plan/velocity.md:41`.
   `tools/nv/test/plan.test.ts` stops using M11 as its fixture. `docs/future-ideas.md` loses
   `nvs convert`. `docs/plan/design.md` is frozen and keeps it.
2. **The rules and the pages.** The eight `tooling/convert-*` rules are deleted, with
   `php-migration/a-trait-converts-to-an-interface-or-a-delegate` and
   `php-migration/a-shell-call-converts-to-process-under-review`. Every live sentence that names
   `nvs convert`, "the converter", "a converted program" or the importer is rewritten whole to say what
   is true without it: `docs/rules/core-api/`, `docs/rules/iteration/`, `docs/rules/programs/`,
   `tooling/fmt-never-inserts-visibility`, `docs/reference/findings.md`, `docs/spec/00-overview.md`,
   `crates/nvs-test/src/` (three files name M11), the comments in `tests/conformance/reject/` and
   `crates/nvs-stdlib/tests/`, and `website/src/content/`. The rules chapter pages under
   `website/src/content/docs/docs/rules/` are rendered, not edited.

- **A deleted rule's citations.** A live citation is rewritten. A frozen record that cites a deleted
  rule (about 22 under `docs/decisions/`) gets the one edit commit 3759c8451 made to ADR 0150: the
  `rule:` prefix goes and the id stays in backticks, so the citation check finds nothing dangling and
  the record still says what it meant.
- **Pinned by** the Stage 5 checks.

## Stage 6 — no list maps PHP to Novis

**Does:** Deletes the PHP-to-Novis name list in every form, and the editor completion, the diagnostic clause, the tool, the tests and the gates that read it.

Three file sets, in this order.

1. **The compiler and the editor.** `crates/nvs-stdlib/src/php_names.rs`, its build-time join in
   `crates/nvs-stdlib/build.rs` and `tools/data/php-builtins.txt` are deleted.
   `nvs_hir::PhpFunctions` (`crates/nvs-hir/src/members.rs:192-209`) and the argument that carries it
   leave `resolve_program_linted` and its two callers (`crates/nvs-cli/src/main.rs:1948`,
   `crates/nvs-lsp/src/document.rs:681`). E0320 always gives its general help. The PHP-name layer leaves
   `crates/nvs-lsp/src/completion.rs` (`php_builtins`, `php_item`, the module doc at `:176-182`), with
   `PhpNames` in `crates/nvs-lsp/src/settings.rs`, the server's two reads and the `nvs.completion.phpNames`
   setting in `editors/vscode/package.json` and its test. `crates/nvs-lsp/tests/completion.rs` loses the
   PHP-name tests. A conformance or `.lspt` case whose frozen output prints E0320's PHP clause or a
   PHP-name item is rewritten from what Novis then prints, **in the commit that removes the feature**,
   and that commit names each case.
2. **The data, the tools and the gates.** `docs/spec/02-php-migration.md`, `data/spec/php-migration.json`
   and its importer half in `tools/nv/import/spec.ts`, `tools/dump-php-builtins.php`,
   `tools/nv/cmd/migration.ts` and the reference topic `tools/nv/cmd/reference.ts:507` generates from it.
   The `Replaces` column leaves `docs/spec/01-core-library.md` and `data/spec/core-members.json`, with
   its readers: `tools/nv/renderers/website-core.ts` (`replaces`, `replacesPhrase`) and the "Replaces in
   PHP" fact in `website/src/components/MethodSignature.astro:201-207`. The tests and gates that held
   the list go with it: `crates/nvs-stdlib/tests/php_names.rs`,
   `every_migration_member_row_has_a_conformance_case` and
   `every_migration_member_row_names_a_registered_member` in
   `crates/nvs-stdlib/tests/spec_registry_coverage.rs`, the CI job `migration-table`
   (`.github/workflows/ci.yml:515-527`), the wrap's `MIGRATION_GATES` (`tools/nv/cmd/session.ts:513`)
   and `core-api/removals`' `guardedBy` on `migration.ts`.
3. **The rules and the prose.** The five PHP-name rules are deleted:
   `php-migration/every-php-builtin-is-a-completion-candidate`,
   `php-migration/the-php-name-layer-is-joined-at-build-time`,
   `php-migration/an-item-inserts-only-a-registered-member`,
   `php-migration/a-php-name-sorts-below-every-novis-symbol` and
   `php-migration/completion-php-names-setting`. `docs/rules/ide/` stops naming the layer.
   `website/src/content/claims/php-migration-is-accounted.md` is deleted. The playbook entries that name
   the list (`data/playbook/tooling/`, `data/playbook/writing-a-test-case/`) are deleted.
   `docs/agent/goals/README.md` § *What stops the run* stops describing `bun nv migration` as the
   parity gate, and that bullet is rewritten whole. `docs/reference/README.md` stops linking the list.

- **Pinned by** the Stage 6 checks.

## Stage 7 — the decisions M11 was holding are Novis's own

**Does:** Decides the three gaps parked on M11 as "a decision on PHP compatibility", by Novis's own rules, and deletes their records.

Three file sets, one per gap: `crates/nvs-stdlib/src/str.rs`, `crates/nvs-stdlib/src/arr.rs`,
`crates/nvs-stdlib/src/bigint.rs`, each with its conformance cases. Each was waiting for the converter
to say what PHP needs. Nothing does now, so each follows the rule Novis already has.

- **A `Core\Str` search matches only on grapheme boundaries**
  (`data/gaps/nvs-stdlib/a-match-inside-a-character-reports-the-wrong-position.json`). Every position in
  a `string` is a grapheme (`rule:types/string-is-utf8`), so a match that starts or ends inside a
  cluster is not a match. That holds for the whole search family: `indexOf`, `lastIndexOf`, `contains`,
  `countOf`, `before`, `after`, `split`, `replace`. The gap's four examples become conformance cases.
- **A member that calls back into Novis code holds its subject for the whole walk**
  (`data/gaps/nvs-stdlib/callback-that-reshapes-the-subject-reshapes.json`). It is what `foreach`
  already does, so a callback that writes to the subject writes to a copy, and the walk reads what it
  started with. The cost is one refcount pair per call of such a member. The attack at
  `tests/hostile/core/Arr/filter/01-predicates-written-to-break-the-walk.nvs` step 4 then ends when the
  walk ends, not at the memory ceiling, and its comment says so.
- **`powMod` returns the least non-negative residue**
  (`data/gaps/nvs-stdlib/powmod-and-pow-followed-by-mod.json`). Modular exponentiation is a different
  operation from `mod`, and the residue is what every use of it wants. `POW_MOD_DOC`'s `ret` states the
  result for a negative receiver, and a conformance case pins `Core\BigInt::of(-3)->powMod(3, 7)` as `1`.
- **These are the agent's calls of 2026-10-01, not the user's.** If the user overturns one before this
  goal is reached, this section is rewritten first.
- **Pinned by** the Stage 7 checks.

## Stage 8 — the rules state the behaviour

**Does:** Rewrites every rule that makes PHP the reference, moves the language rules out of `php-migration/` and deletes the topic, and deletes the `divergesFromPhp` field and `docs/divergences.md`.

File sets by rule topic, one topic per group: `php-migration/` first, then `types/`, `classes/` and
`expressions/`, then `config/`, `errors/`, `tooling/`, `packaging/`, `testing/`, `observability/`,
`core-classes/`. Each fragment is rewritten whole, and its chapter is rendered again.

- **The `php-migration/` topic is dissolved.** Each remaining rule that states Novis behaviour moves to
  the topic its behaviour belongs to, with an id that names the behaviour and a body rewritten without
  PHP: `absent-storage-is-never-a-zero-value`, `an-element-write-needs-storage-to-write-back-into`,
  `a-body-never-falls-off-its-end`, `no-return-leaves-a-finally`, `a-constructor-return-carries-no-value`,
  `a-readonly-property-declares-no-default`, `a-declared-type-answers-before-the-program-runs`,
  `let-and-is-are-reserved`, `one-type-test`, `no-partial-application`,
  `a-session-id-the-store-did-not-issue-is-rejected`. A rule whose whole content is a relation to PHP is
  deleted: `every-divergence-is-deliberate-and-listed`, and `a-deprecation-is-a-refusal` unless what it
  refuses can be stated without PHP. Then `data/rules/php-migration.json` and its chapter are deleted.
  Every live citation follows the move (about 100 files, `git grep -l "php-migration/"`); a frozen one
  gets Stage 5's edit, or the new id where the rule moved.
- **`divergesFromPhp` is deleted** from `tools/nv/schema/rule.ts`, the importer
  (`tools/nv/import/rules.ts`), `tools/nv/cmd/rules.ts`, `tools/nv/renderers/website-rules.ts`
  (`:184,196,290-297`, the per-rule PHP flag and aside), `website/src/components/RuleBookIndex.astro:48`
  (the counter), `tools/nv/test/import.test.ts`, and the about 593 records that carry it — one script
  under `.agent-tmp/`, deleted after. `docs/divergences.md` and the generator that writes it are
  deleted, and every link to it is rewritten.
- **Normative lines** say the behaviour itself: `types/arithmetic.md:5` ("PHP-exact"),
  `types/integer-literals.md:12`, `types/arrays.md:10`, `types/declaration.md:18`,
  `types/string-is-utf8.md:18`, `classes/clone-is-shallow.md:2`, `config/ceilings-are-their-own-directives.md:20`,
  `errors/throw-is-not-slower.md:8`, `tooling/fmt-is-one-canonical-style.md:9`,
  `config/opcache-file-cache-directives-are-system.md:4`.
- **The testing rules**: `testing/nvst-is-separate`, `testing/probes-on-is-a-tested-configuration`,
  `ide/case-files-have-their-own-grammar` and `packaging/the-sweep-is-fired-by-a-human` lose the oracle.
- **Incidental mentions** (about 35 lines, e.g. `classes/two-copy-depths.md:11`,
  `packaging/extension-calls-are-statically-typed.md:8`) are cut where the sentence is complete without
  PHP. No rule fragment compares with PHP any more: the comparison lives on the concept page.
- **Code fences tagged `php` that hold Novis code** (about 47) are retagged `nvs`. A fence that holds
  real PHP code is deleted with the comparison it served.
- **Pinned by** the Stage 8 checks.

## Stage 9 — the reader's text steps back, and one concept page stays

**Does:** Takes "This replaces PHP's `x`." out of `about.md` and the registry text, takes PHP off the pages that do not compare, and cuts the php-differences chapter to one concept page.

Five file sets, in this order.

1. **`about.md`.** About 244 files under `docs/examples/`, outside `docs/examples/tools/php-differences/`,
   carry the line. It is deleted. Where it was the first sentence, the file opens with what the feature
   does. `docs/examples/README.md:55` no longer invites the hint.
2. **The registry text** in `crates/nvs-stdlib/src/*.rs` (`short:`, `desc:` and `ret:` strings; about
   120 lines, most in `math.rs` and `str.rs`). The same sentence is deleted. Text that is real interop
   (a password hash PHP made) stays.
3. **The website.** `website/src/components/MethodLead.astro:24` (the PHP fallback lead sentence), the
   home page's "PHP developers feel at home" card (`website/src/content/docs/index.mdx:62-70`) and
   `website/src/content/docs/docs/getting-started/hello-world.mdx:28,48`.
   `website/src/content/docs/why-novis.mdx` and the claims pages that compare the languages stay. The
   home page and `README.md:6` keep one honest line: Novis grew out of PHP's ideas.
4. **The concept page.** `docs/reference/tools/30-php-differences.md` keeps its id and becomes one
   short page for somebody who knows PHP: how Novis thinks differently (today's *The short list*,
   kept as concepts), what Novis can do that PHP cannot, and **how to port a program** — read the old
   program for what it does, then write it again with Novis's own features; for a large codebase, give
   the work to an AI agent that reads `nvs agent`'s topics. It says plainly that there is no converter
   and no name table, and why. The refusal tables, *What parses but behaves differently*, *Two
   spellings side by side* and the crosswalk are deleted. Its keywords and summary are rewritten to
   match. `docs/examples/tools/php-differences/` keeps the folders whose section is still on the page,
   enough for the feature's proofs, and its `about.md` is rewritten whole. A `covers:` marker in
   `tests/conformance/` that pointed at a deleted section points at the section that remains or is
   removed; the case itself stays. `crates/nvs-cli/src/agent.rs:156` moves `php-differences` out of the
   first four primer topics, and the topic stays reachable by name.
5. **The rest of the reference and the binary.** The lang chapters lose their PHP comparison tables
   and passing mentions. `README.md:24-30` is trimmed to one line that links the concept page. The four
   comments in `benches/members/` lose PHP.

- **Pinned by** the Stage 9 checks.

## Stage 10 — rendered and proven

**Does:** Renders every generated file again and proves the roster still owes nothing.

- `bun nv reference` writes `docs/novis.md` again. The rules, the ground rules, the decision index and
  the website data are rendered again.
- **Pinned by** the Stage 10 checks.

## Standing decisions

- **The user's calls of 2026-10-01, first set.** No test and no check runs PHP to judge Novis. The
  sentence "This replaces PHP's `x`." leaves `about.md` and the registry text. **The PHP benchmarks
  stay**: `benches/userland`'s PHP engine, `bun nv bench --serve-vs-fpm` and `benches/proxied`'s
  php-fpm arm, with their ledgers and rules. `nvs test --php` and the `--ORACLE--` sections are
  removed for users too. **Priority 2 is deleted, not reworded.** Correctness is the job of the rules
  and the conformance cases. The ordering decides between the four things left in it.
- **The user's calls of 2026-10-01, second set.** There is no mechanical migration help from PHP to
  Novis, planned or shipped. M11 is scratched. A program is ported by rethinking it, by hand or with an
  AI agent and `nvs agent`. **No list maps PHP functions or constructs to Novis ones**, in a document,
  in data or in a feature: the editor's PHP-name completion and E0320's PHP clause go with the list.
  **One concept page stays**, the php-differences chapter cut down, and the divergence catalogue
  (`docs/divergences.md`, `divergesFromPhp`) goes. The language rules filed under `php-migration/` move
  to their own topics.
- **Origins stay honest, not prominent.** One line on the home page and in `README.md` says Novis grew
  out of PHP's ideas. PHP is named elsewhere only on the concept page and its examples and attacks,
  `why-novis`, the claims pages that compare the languages, the PHP benchmarks, and the diagnostics for
  a PHP habit (E0229 `<?php`, W1004, the superglobals and PHP-constant cards). Those diagnostics stay
  because each is help for one error a person meets, not a table to translate from — the agent's call,
  not the user's.
- **Frozen records are not edited**: `docs/decisions/`, `docs/adr/`, `docs/plan/design.md` and every
  walked goal's prose. Two edits are allowed. A `validatedBy` pointer at a deleted case points at the
  conformance case that pins the same behaviour; if the record schema refuses the edit, the pointer is
  dropped and the commit says so. A `rule:` citation of a moved rule takes its new id, and one of a
  deleted rule loses the `rule:` prefix (Stage 5).
- **Names stay**: the `php-differences` id and paths, the `E_PHP_*` constants, the Cargo keyword. A
  conformance case whose file name mentions PHP is not renamed.
- **Rust comments are not swept.** A comment in a file a slice already edits that says "as PHP does" is
  rewritten whole to state the behaviour. No pass walks `crates/` for them.
- **Conformance output is never edited to make a check pass.** A case added in Stage 3 freezes what Novis
  prints today. The one exception is Stage 6's: a removed feature's text leaves the cases that print it,
  in the commit that removes it.
- **One ADR slot**: one new record, checked right before it is written, in Stage 2. Its tradeoffs:
  performance — `nv verify` runs one tree fewer and spawns no PHP; the stdlib build loses its join; a
  `Core\Str` search checks a grapheme boundary per candidate match, and a callback-taking `Core\Arr`
  member pays one refcount pair per call. Memory — the binary loses the PHP-name table. Usability —
  somebody porting a program loses side-by-side `--ORACLE--` tests, PHP-name completion, E0320's PHP
  clause and the construct tables; they gain one page that says how Novis thinks and how to port by
  rethinking, and the agent topics stay. Simplicity — one runner mode, two sections, one flag and one
  embedded grammar fewer, about 280 cases and about 30 playbook entries fewer, one milestone, about
  fifteen rules and a whole rule topic fewer, a 1,788-line list, a 1,430-line inventory, an editor
  layer and its setting fewer, and a field on about 593 records fewer. PHP is needed only to run the
  benchmarks.
- **Every comment in a changed `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write.**
