# Handoff

## State

**Goal `workspace-index`, stage 4 is complete: the three requests that need no new reading are
answered, and the fourth is refused on the record.** `textDocument/signatureHelp` is
`nvs_lsp::hover::help_at`, which walks outward to the call the cursor is inside and renders its row
through the same `signature` that writes a `Core` member's hover line — so
`Core\Str::length(string $s): uint` and `Adder::add(int $a, int $b): int` are one renderer's output
and a parameter list has no second spelling. `textDocument/typeDefinition` is
`nvs_lsp::definition::type_at`, following the type the checker recorded rather than the name the
cursor is on; `textDocument/implementation` is the index's `subtypes` query answered as a jump list.
`textDocument/declaration` is declared nowhere and named nowhere, and
`declaration_is_not_answered_because_it_would_answer_identically` holds both halves.

**The stage's file set was one file short.** `crates/nvs-lsp/src/definition.rs` is not in what the
last handoff named, and the type navigation cannot live anywhere else: `Target` and `site` are
`pub(crate)`, so a walk from a `TypeId` to a declaration has to be written beside them.

Nothing is blocked. The goal's one ADR is still unopened, and stage 4's admissions against
[ADR 0099](../decisions/0099.md) § 3's test are now part of what it owes, beside stage 2's index
shape and stages 5 and 6 — it is written once all four have landed, because a record is frozen on
acceptance and cannot be amended into. Its `changes:` block still owes
`rule:ide/the-request-set-is-closed` the amendment naming M10's additions beside M4B's nine:
`documentHighlight`, and now `signatureHelp`, `typeDefinition` and `implementation`.

## Next group

**Stage 5: the completion arms, and the enumeration that gates them** —
`crates/nvs-lsp/src/completion.rs`'s `asked`/`position` split is where both new arms go, so the two
of them and their guard are one reading. One file set: `crates/nvs-lsp/src/completion.rs`,
`crates/nvs-lsp/src/capabilities.rs`, `crates/nvs-lsp/src/index.rs`, `crates/nvs-lsp/tests/`.

- [ ] **The namespace arm.** A cursor after a namespace separator offers the registry's classes and
      the index's declarations under that prefix and **no keywords**, dispatched in
      `crates/nvs-lsp/src/completion.rs:198`'s `at` before `crates/nvs-lsp/src/completion.rs:361`'s
      `position` catch-all, reading the prefix off `crates/nvs-lsp/src/completion.rs:466`'s
      `namespace_at`. `\` is already a declared trigger character at
      `crates/nvs-lsp/src/capabilities.rs:209`, so until this arm exists a cursor after `Core\` is
      answered the position list — which is the other half of what this closes.
      `a_namespace_segment_offers_the_registry_and_the_index_and_no_keywords` and
      `every_trigger_character_reaches_an_arm_that_is_not_the_position_list`.
      `rule:ide/completion-offers-only-what-the-compiler-derived`.
- [ ] **The bare-name arm.** An identifier at a top-level position offers the imports in force —
      `crates/nvs-lsp/src/completion.rs:499`'s `imports_of` is that map already — and every
      declaration `crates/nvs-lsp/src/index.rs:401`'s `declaration` side holds, beside the keywords
      the position already offers rather than instead of them.
      `a_bare_name_offers_the_imports_in_force_and_the_declarations_in_the_index`.
      `rule:ide/completion-offers-only-what-the-compiler-derived`.
- [ ] **The enumeration guard.** A test over `crates/nvs-lsp/src/completion.rs`'s own source that
      names every source a completion item's value comes from and fails one that is not a table the
      compiler builds for another reason — a directory walk, an annotation dialect, a network call —
      in the shape `crates/nvs-lsp/tests/index.rs:209`'s reader enumeration uses.
      `every_completion_source_names_a_compiler_table` and
      `no_completion_source_reads_a_directory_layout_or_the_network`.
      `rule:ide/completion-offers-only-what-the-compiler-derived`.

## Backlog

- Stage 5's second check is `-p nvs-stdlib`: the PHP-name table as one build-time join —
  `rule:php-migration/every-php-builtin-is-a-completion-candidate`.
- Stage 6 is inlay hints, bounded to the two settled shapes — `docs/agent/loop-goal.toml:7824`.
- The goal's one ADR, after stage 6: stages 2/4/5/6 in one record, amending
  `rule:ide/the-request-set-is-closed` — `docs/agent/loop-goal.md` § *Standing decisions*.
- An inherited member is still not offered by completion, and visibility is still not applied —
  `crates/nvs-lsp/src/completion.rs`'s `# Known gaps`.
