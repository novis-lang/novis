# Handoff

## State

**Goal `workspace-index`, stage 5's completion arms are landed: the namespace arm, the bare-name arm
and the enumeration that gates them.** A cursor after a separator is `nvs_lsp::completion`'s
`Asked::Namespace`, offered the registry's classes and enums and the workspace index's declarations
under that prefix and no keyword; a bare name at a statement position is offered the `use`
declarations in force and every type the index holds, each labelled with the shortest spelling that
resolves at that cursor, beside the words the position already offered rather than instead of them.
`every_completion_source_names_a_compiler_table` and
`no_completion_source_reads_a_directory_layout_or_the_network` read the module's own source and hold
`rule:ide/completion-offers-only-what-the-compiler-derived` the way that rule asks to be held.

**`completion` is the first module outside `server.rs` to read the one index**, through
`SymbolIndex::files` and `SymbolIndex::declarations_in` — queries `textDocument/codeLens` already
makes, so `rule:ide/five-features-are-one-reference-index`'s five *queries* are unchanged and its
count of readers now understates by one. That is a fact the goal's ADR owes a paragraph, not a rule
change; nothing is blocked on it.

Stage 5's other check is untouched: the PHP-name layer is the next group. The goal's one ADR is
still unopened and now owes stage 5's two arms beside stage 2's index shape and stage 4's request
admissions, with the same `changes:` amendment to `rule:ide/the-request-set-is-closed`.

## Next group

**Stage 5 (continued): the PHP-name layer, which is one build-time join and the four item shapes it
decides** — all three slices read the same three inputs, so the join and what it refuses are one
reading. One file set: `crates/nvs-stdlib/` (a `build.rs` that does not exist yet),
`tools/data/php-builtins.txt`, `docs/spec/02-php-migration.md`, `crates/nvs-lsp/src/completion.rs`.

- [ ] **The build-time join.** One generated table in `nvs-stdlib` joining the oracle inventory at
      `tools/check-migration.py:42`'s `tools/data/php-builtins.txt`, the migration rows whose four
      outcomes are `docs/spec/02-php-migration.md:23`, and the registry lookup at
      `crates/nvs-stdlib/src/registry.rs:2554`'s `class` — failing the build on a destination
      spelling that matches no registry member. The crate has **no `build.rs` today**, which is the
      first thing this slice writes. `every_php_builtin_in_the_oracle_inventory_is_a_candidate` and
      `a_destination_spelling_that_matches_no_registry_member_fails_the_build`.
      `rule:php-migration/every-php-builtin-is-a-completion-candidate`.
- [ ] **The four shapes, three of which insert nothing.** Asserted over the generated table's own
      rows rather than over a rendered item, which is why the check files it under `-p nvs-stdlib`:
      the table is what says whether a row has an insertable destination, and "inserts nothing" is
      the absence of one and not an empty string. The destination is registered exactly when
      `crates/nvs-stdlib/src/registry.rs:2554`'s `class` answers, which is the one shape of the four
      that inserts. `three_of_the_four_item_shapes_insert_nothing`.
      `rule:ide/three-of-four-item-shapes-insert-nothing`.
- [ ] **The arm and its setting.** The candidates offered beside the bare names at
      `crates/nvs-lsp/src/completion.rs:607`'s `in_reach`, gated on `nvs.completion.phpNames`
      (`all`/`resolved`/`off`, default `all`) added to `crates/nvs-lsp/src/settings.rs:44`'s
      `Settings` and to the extension's frozen roster.
      `rule:ide/contributions-are-frozen-and-only-ever-added`.

## Backlog

- Stage 6, inlay hints, reverses [ADR 0099](../decisions/0099.md) § 3's deferral — `loop-goal.md` § *Stage 6*.
- The goal's one ADR is unopened and owes four stages' reasoning — `loop-goal.md` § *Standing decisions*.
- Completion is a sixth *reader* of the one index and makes no sixth query — that ADR's paragraph.
- A cursor in a type annotation or a parameter list reaches no arm — `completion.rs` § *Known gaps*.
- A namespace's declarations are only the ones the index holds, so the default scope narrows them — same.
- `registry`'s six global interfaces are reachable by no arm: they have no namespace to be under — same.
