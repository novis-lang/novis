# Handoff

## State

**Goal `workspace-index`, stage 5's PHP-name table is landed.** `crates/nvs-stdlib/build.rs` — the
crate's first build script — joins the oracle inventory and the migration table into
`nvs_stdlib::php_names::CANDIDATES`, one row per name the inventory lists, sorted by its PHP
spelling so a completion arm can take a prefix range off it. `php_names::Item` is
`rule:ide/three-of-four-item-shapes-insert-nothing`'s four rows as an enum, and `insertion()`
returns an `Option` so "inserts nothing" is the absence of an edit.

**The registry is the third input and the build script cannot reach it**, so the join is two-stage:
the documents in `build.rs`, `registry::class` in `php_names::Destination::is_registered`, and the
refusal in `crates/nvs-stdlib/tests/php_names.rs` — which fails on a member spelling a *registered*
class does not declare, the typo case nothing else in the tree distinguishes. A destination whose
class the registry does not declare at all is the second item shape, not a failure. Both files'
module docs own that split; the goal's ADR owes it a paragraph beside stage 2's index shape, stage
4's request admissions and stage 5's two arms.

**`nvs-stdlib` now fails to build without `docs/spec/02-php-migration.md` and
`tools/data/php-builtins.txt` on disk**, which is a deliberate difference from
`crates/nvs-cli/build.rs`, whose own doc says nothing there may fail a build.

## Next group

**Stage 5 (closing): the PHP-name arm and the setting that gates it** — one file set:
`crates/nvs-lsp/src/completion.rs`, `crates/nvs-lsp/src/settings.rs`,
`crates/nvs-lsp/tests/completion.rs`, `editors/vscode/package.json`.

- [ ] **The arm.** The candidates `nvs_stdlib::php_names::starting_with` answers for, offered beside
      the bare names at `crates/nvs-lsp/src/completion.rs:606`'s `in_reach` — one item per
      `Candidate::items()` entry, with `Item::insertion()` and nothing else reaching the buffer.
      `rule:php-migration/an-item-inserts-only-a-registered-member`.
- [ ] **The setting.** `nvs.completion.phpNames` (`all`/`resolved`/`off`, default `all`) on
      `crates/nvs-lsp/src/settings.rs:44`'s `Settings`, and in the extension's frozen roster in
      `editors/vscode/package.json`. `crates/nvs-lsp/tests/extension_reference.rs:82`'s
      `the_chapter_documents_every_contributed_setting_and_no_other` wants the reference chapter
      in the same commit. `rule:ide/contributions-are-frozen-and-only-ever-added`.
- [ ] **The source, enumerated.** `crates/nvs-lsp/tests/completion.rs:364`'s
      `every_completion_source_names_a_compiler_table` is what the new arm has to satisfy: the
      table it names is `nvs_stdlib::php_names::CANDIDATES`, which the compiler builds from two
      audited documents rather than from a convention scan.
      `rule:ide/completion-offers-only-what-the-compiler-derived`.

## Backlog

- Stage 6, inlay hints, reverses [ADR 0099](../decisions/0099.md) § 3's deferral — `loop-goal.md` § *Stage 6*.
- The goal's one ADR is unopened and owes five stages' reasoning — `loop-goal.md` § *Standing decisions*.
- A `NotRegistered` item cannot name the milestone it waits on: only `tests/migration-members-outstanding.txt` knows, and it is a test fixture — `php_names.rs` module doc.
- A destination naming an interface (`Core\Db\Queryable`) inserts nothing, since the interface has no registry row — same.
- A cursor in a type annotation or a parameter list reaches no arm — `completion.rs` § *Known gaps*.
- `registry`'s six global interfaces are reachable by no arm: they have no namespace to be under — same.
