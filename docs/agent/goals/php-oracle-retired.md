---
milestone: post-parity
position: last
---
# Loop goal 187 — nothing in Novis is tested against PHP, and PHP is named only where a page compares or helps a migration

PHP was the oracle that let the language be prototyped fast: a case ran under both engines, and a
difference was either a bug or a recorded divergence. That era is over. Novis has diverged a long way
from PHP, and every behaviour is now stated by its own rule and pinned by its own conformance case.
This goal removes every test and every comparison that runs PHP to judge Novis, and it moves PHP out
of the prose that does not compare anything.

What stays is a **document**: the list of what PHP has and what Novis has in its place, for the
migration helper planned later. Nothing tests that list for completeness against PHP any more.

```text
removed   tests/differential/ (281 cases), the --ORACLE-- and --ORACLE-DIVERGES-- sections, `nvs test --php`,
          bun nv try's PHP twin, the editor grammar's embedded PHP, the oracle playbook entries,
          the completeness tests over the PHP inventory and the `migration-table` CI job
kept      docs/spec/02-php-migration.md, tools/data/php-builtins.txt and its generator, docs/divergences.md,
          the php-migration rule chapter, the php-differences reference chapter and its examples and attacks,
          the PHP-name completion in the editor, the PHP benchmarks (benches/userland, --serve-vs-fpm, benches/proxied)
rewritten priority 2 (AGENTS.md and rule:programs/memory-priority), the rules that make PHP normative,
          about.md and registry text that opens with or ends on "This replaces PHP's `x`."
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
here — no case is deleted from it, and no frozen output in it is edited.

## Stage 2 — priority 2 states the behaviour, not PHP

**Does:** Writes the decision record and rewrites the priority line and the rules that ground themselves in it.

One file set: `AGENTS.md`, `data/rules/programs/memory-priority.json` and its fragment,
`data/rules/php-migration/every-divergence-is-deliberate-and-listed.json` and its fragment,
`website/src/content/docs/docs/index.mdx`, `docs/agent/commands.md`, `docs/agent/user-report.md`.

- **The decision record**, written first, for the whole goal. It `modifies` ADR 0004's priority list
  through `programs/memory-priority`, and `php-migration/every-divergence-is-deliberate-and-listed`,
  `testing/nvst-is-separate` and `ide/case-files-have-their-own-grammar`. It states the user's calls
  and the tradeoffs from § *Standing decisions*.
- **Priority 2** becomes "Correctness of language semantics — the observable behaviour the rules
  state". `AGENTS.md:51`, `docs/rules/programs/memory-priority.md:5` and
  `website/src/content/docs/docs/index.mdx:25` say the same words. This also ends the contradiction with
  `rule:programs/no-compatibility-promise`, which forbids "PHP compatible" in any document.
- **`php-migration/every-divergence-is-deliberate-and-listed`** is grounded in migration help: the list
  exists so somebody moving a program knows what changes. It no longer cites priority 2 or a `.phpt`
  pass rate.
- `docs/agent/commands.md:418` and `docs/agent/user-report.md:54` stop naming PHP behaviour as what a bug
  is measured against. A bug is Novis doing what a rule or the reference says it does not.
- The playbook bullet whose `[until:]` is keyed on the old `AGENTS.md` text
  (`data/playbook/tooling/a-next-group-items-rationale-about-php-behaviour-is-a.json`) retires with it.
  That is intended.
- **Pinned by** the Stage 2 check.

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

**Does:** Deletes `tests/differential/`, the oracle sections, `nvs test --php` and every reader of them.

Three file sets, in this order.

1. **The runner.** `crates/nvs-test/src/case.rs` (`:31-36`, `:342-343`, `:526-563`),
   `crates/nvs-test/src/run.rs` (`php_available` at `:183`, the skip at `:196`, the comparison at
   `:337-355`, `run_php` at `:444`), `crates/nvs-test/src/lib.rs:347-351`,
   `crates/nvs-cli/src/main.rs:463-465` and `:3198`. `--ORACLE--` and `--ORACLE-DIVERGES--` become
   unknown sections, refused with the error every unknown section gets. The `Oracle` enum, the `--php`
   flag and `php_available` are deleted. `crates/nvs-test/src/expect.rs:291`'s test is renamed for what
   the wildcards match, not for PHP. `crates/nvs-lsp/src/case.rs:639-641` keeps refusing the section in
   an `.lspt`, because every unknown section is refused there.
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

## Stage 5 — the migration list is a document

**Does:** Removes the tests that hold the PHP list complete, and keeps the tests that a row names a real Novis method.

One file set: `crates/nvs-stdlib/tests/php_names.rs`, `crates/nvs-stdlib/tests/spec_registry_coverage.rs`,
`.github/workflows/ci.yml`, `tools/nv/cmd/session.ts`, `tools/nv/cmd/migration.ts`,
`data/rules/core-api/removals.json`, `data/rules/php-migration/every-php-builtin-is-a-completion-candidate.json`.

- **Deleted:** `every_php_builtin_in_the_oracle_inventory_is_a_candidate`
  (`crates/nvs-stdlib/tests/php_names.rs:132`), `every_migration_member_row_has_a_conformance_case`
  (`crates/nvs-stdlib/tests/spec_registry_coverage.rs:1980`), the CI job `migration-table`
  (`.github/workflows/ci.yml:515-527`), the wrap's `MIGRATION_GATES` (`tools/nv/cmd/session.ts:513`),
  and `core-api/removals`' `guardedBy` on `migration.ts`.
- **Kept:** `every_migration_member_row_names_a_registered_member` and every test of the completion
  feature's behaviour (`crates/nvs-lsp/tests/completion.rs`, `php_names.rs`'s other tests). A
  renamed Novis method must never leave a dead name in the editor or the table.
- `bun nv migration` stays as a report somebody runs by hand. Nothing gates on it.
  `docs/agent/goals/README.md` § *What stops the run* stops describing it as a gate ("every one of them
  cased"), and that bullet is rewritten whole.
- `php-migration/every-php-builtin-is-a-completion-candidate` says the inventory is made by hand with
  `tools/dump-php-builtins.php`, not "the same PHP build the differential suite uses as its oracle".
- **Pinned by** the Stage 5 checks.

## Stage 6 — the rules state the behaviour

**Does:** Rewrites every rule fragment that makes PHP the reference, and trims PHP from the ones that only mention it in passing.

File sets by rule topic, one topic per group: `types/`, then `classes/` and `expressions/`, then
`config/`, `errors/`, `tooling/`, `packaging/`, `testing/`, `observability/`, `core-classes/`. Each
fragment is rewritten whole, and its chapter is rendered again.

- **Normative lines** say the behaviour itself: `types/arithmetic.md:5` ("PHP-exact"),
  `types/integer-literals.md:12`, `types/arrays.md:10`, `types/declaration.md:18`,
  `types/string-is-utf8.md:18`, `classes/clone-is-shallow.md:2`, `config/ceilings-are-their-own-directives.md:20`,
  `errors/throw-is-not-slower.md:8`, `tooling/fmt-is-one-canonical-style.md:9`,
  `config/opcache-file-cache-directives-are-system.md:4`. The `php-migration/` fragments keep their
  comparisons, because comparing is their subject.
- **The testing rules**: `testing/nvst-is-separate`, `testing/probes-on-is-a-tested-configuration`,
  `ide/case-files-have-their-own-grammar` and `packaging/the-sweep-is-fired-by-a-human` lose the oracle.
  `tooling/convert-equivalent-is-proven`, `tooling/convert-php-front-end` and
  `tooling/fmt-never-inserts-visibility` say a converted program is proven by a conformance case with
  hand-written expected output, and nothing runs PHP.
- **Incidental mentions** (about 35 lines, e.g. `classes/two-copy-depths.md:11`,
  `packaging/extension-calls-are-statically-typed.md:8`) are cut where the sentence is complete without
  PHP. A "PHP does X; Novis does Y" line stays: it is a comparison, and `docs/divergences.md` is built
  from the `divergesFromPhp` field beside it.
- **Code fences tagged `php` that hold Novis code** (about 47) are retagged `nvs`. A fence that holds
  real PHP code, in a comparison, keeps its tag.
- **Pinned by** the Stage 6 check.

## Stage 7 — the reader's text steps back

**Does:** Takes "This replaces PHP's `x`." out of `about.md` and the registry text, and takes PHP off the pages that do not compare.

Four file sets, in this order.

1. **`about.md`.** About 244 files under `docs/examples/`, outside `docs/examples/tools/php-differences/`,
   carry the line. It is deleted. Where it was the first sentence, the file opens with what the feature
   does. `docs/examples/README.md:55` no longer invites the hint.
2. **The registry text** in `crates/nvs-stdlib/src/*.rs` (`short:`, `desc:` and `ret:` strings; about
   120 lines, most in `math.rs` and `str.rs`). The same sentence is deleted. The structured `replaces`
   data stays: it feeds the migration table, `docs/spec/01-core-library.md`'s column and the
   php-differences chapter. Text that is real interop (a password hash PHP made) stays.
3. **The website.** `website/src/components/MethodLead.astro:24` (the PHP fallback lead sentence),
   `website/src/components/MethodSignature.astro:203` (the "Replaces in PHP" fact),
   `tools/nv/renderers/website-rules.ts:184,196,290-297` (the per-rule PHP flag and aside),
   `website/src/components/RuleBookIndex.astro:48` (the counter), the home page's "PHP developers feel
   at home" card (`website/src/content/docs/index.mdx:62-70`) and
   `website/src/content/docs/docs/getting-started/hello-world.mdx:28,48`. `website/src/content/docs/why-novis.mdx`
   and the claims pages compare on purpose
   and stay. The home page and `README.md:6` keep one honest line: Novis grew out of PHP's ideas.
4. **The reference and the binary.** `docs/reference/tools/30-php-differences.md:7` no longer opens with
   "Novis is the PHP you already know". `crates/nvs-cli/src/agent.rs:147,156` moves `php-differences`
   out of the first four primer topics. The lang chapters keep their comparison tables and lose the
   passing mentions. `README.md:24-30` is trimmed to the comparison and `nvs convert`. The four comments
   in `benches/members/` lose PHP.

- **Pinned by** the Stage 7 checks.

## Stage 8 — rendered and proven

**Does:** Renders every generated file again and proves the roster still owes nothing.

- `bun nv reference` writes `docs/novis.md` again. The rules, the ground rules, `docs/divergences.md`
  and the website data are rendered again.
- **Pinned by** the Stage 8 check.

## Standing decisions

- **The user's calls of 2026-10-01.** No test and no check runs PHP to judge Novis. The list of what PHP
  has and what Novis has is a document for the later migration helper. The tests that hold that list
  *complete* against PHP are deleted. The tests that a row names a real Novis method stay, because the
  PHP-name completion is a shipped editor feature. The sentence "This replaces PHP's `x`." leaves
  `about.md` and the registry text; the structured `replaces` data stays and is shown only in the
  migration table and the php-differences chapter. **The PHP benchmarks stay**: `benches/userland`'s
  PHP engine, `bun nv bench --serve-vs-fpm` and `benches/proxied`'s php-fpm arm, with their ledgers and
  rules. `nvs test --php` and the `--ORACLE--` sections are removed for users too.
- **Origins stay honest, not prominent.** One line on the home page and in `README.md` says Novis grew
  out of PHP's ideas. PHP is named elsewhere only where a page compares features or helps a migration:
  the php-migration chapter, the php-differences chapter, its examples and attacks, `docs/divergences.md`,
  `why-novis`, the claims pages and the diagnostics that help somebody moving code (E0229 `<?php`,
  W1004, the superglobals and PHP-constant cards).
- **Frozen records are not edited**: `docs/decisions/`, `docs/adr/`, `docs/plan/design.md` and every
  walked goal's prose. A record's `validatedBy` pointer at a deleted case is the one edit allowed, and it
  points at the conformance case that pins the same behaviour; if the record schema refuses the edit,
  the pointer is dropped and the commit says so.
- **Names stay**: `divergesFromPhp`, the `php-migration` topic, `nvs.completion.phpNames`, the
  `php-differences` paths, the `E_PHP_*` constants, the Cargo keyword. A conformance case whose file
  name mentions PHP is not renamed.
- **Rust comments are not swept.** A comment in a file a slice already edits that says "as PHP does" is
  rewritten whole to state the behaviour. No pass walks `crates/` for them.
- **Conformance output is never edited to make a check pass.** A case added in Stage 3 freezes what Novis
  prints today.
- **One ADR slot**: one new record, checked right before it is written, in Stage 2. Its tradeoffs:
  performance — none on the request path, and `nv verify` runs one tree fewer and spawns no PHP.
  Memory — none. Usability — somebody porting a program loses side-by-side `--ORACLE--` tests; the
  migration helper decides what replaces them. Simplicity — one runner mode, one flag and one embedded
  grammar fewer, about 280 cases and about 27 playbook entries fewer, and PHP is needed only to run the
  benchmarks.
- **Every comment in a changed `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write.**
