# Handoff

## State

**Goal `class-scoped-types`, stage 4 is half landed: the formatter, `nvs meta --json` and two of the
four LSP requests see the member.**

- **The formatter needed no change.** A `type` member is a declaration the syntax index places, so
  `indent.rs` gives it its body's depth in a class, an interface and an enum alike, and the Allman
  brace of its owner is `brace.rs`'s already. The runs *inside* the type expression are left alone —
  `{total: decimal}` in type position is outside `space.rs`'s one-space rule, which is
  `crates/nvs-fmt/src/lib.rs`'s own known gap 5 and the same answer the file-scope form gets.
- **`nvs meta --json` nests an alias under its owner**, in a `types` array of the body's own, in the
  card shape the file-scope roster already writes and named `Owner::Name`. An alias never joins
  `members`, and a body owning none has no `types` key.
- **The LSP outline and its semantic tokens see the member**: both were one missing
  `ClassMemberKind::TypeAlias` arm in a body walk, and both take the kind the file-scope form takes.
- **Stage 3's dedup left one flake behind, now fixed**: `Diagnostics` derived `Debug` over the
  `HashSet` the dedup keeps, so two sinks holding the very same diagnostics printed differently and
  `the_strict_entry_point_reports_exactly_what_it_reported_before` failed at random. The sink writes
  its own `Debug` now and the set is left out of it.
- **Left in stage 4: go-to-definition and completion, which share one missing mechanism** — nothing
  in `nvs-lsp` can tell what *type* a cursor is inside. See the playbook bullet under *Writing Novis
  itself*; it is the first item below. Stage 5 (the rule and the record) is untouched. Nothing is
  blocked.

## Next group

**Stage 4: the tooling sees the member, phase-gated** — one file set: `crates/nvs-lsp/src/definition.rs`,
`crates/nvs-lsp/src/completion.rs`.

- [ ] **A cursor in a written type answers which type it is in** — `crates/nvs-lsp/src/definition.rs:484`
      (`clause_at`) is the shape to copy: the entry file's declarations, scanned for the `Type` spans
      covering the offset, because `nvs_syntax::walk` builds no node for a type. Enumerate the type
      positions a signature has (parameter, return, property, typed local, an alias's own right-hand
      side) rather than every one the grammar allows, and say in the module doc which those are.
      `rule:ide/the-index-answers-the-cursor`.
- [ ] **Go-to-definition on `Owner::Name` in type position lands on the member** —
      `crates/nvs-lsp/src/definition.rs:189` (`Target`) gains an alias arm and
      `crates/nvs-lsp/src/definition.rs:237` (`MemberKind`) the list it is looked for in, so
      `crates/nvs-lsp/src/definition.rs:262` (`site`) answers the member's own name span through
      `declared_member`. Test `definition_of_owner_name_in_type_position_is_the_member`.
      `rule:types/type-alias`.
- [ ] **Completion after `Owner::` in type position offers the alias** —
      `crates/nvs-lsp/src/completion.rs:321` (`asked`) decides off an access *node*, which type
      position has none of, and `crates/nvs-lsp/src/completion.rs:975` (`declared_members`) is where
      the owner's members are listed. Test
      `completion_after_owner_double_colon_in_type_position_offers_the_alias`.
      `rule:types/type-alias`.

## Backlog

- Stage 5: one new record and the `types/class-scoped-alias` fragment — `docs/agent/loop-goal.md` § *Stage 5*.
- `[context] modules` did not print `crates/nvs-lsp/src/document.rs` (`Analysed`, which every request
  reads) or `crates/nvs-lsp/src/render.rs` (how a symbol kind and a token are spelled in a test's
  expected string); both were needed to write stage 4's LSP work.
- `crates/nvs-fmt/src/lib.rs` known gap 5 now has a second reader: a shape type in type position is
  unspaced at both declaration sites — `docs/rules/tooling/fmt-novis-constructs.md` claims the space.
