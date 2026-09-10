# Handoff

## State

**Goal `workspace-index`, stage 0 is closed and green.** A receiver whose access ends at the cursor now
offers its class's members: the parser has no token to make a member name from before a `}`, a `)`, a `]`
or the end of the file, so the access stops at the arrow and `SyntaxIndex::at`'s half-open containment put
the cursor outside every node of it. `crates/nvs-lsp/src/completion.rs:246` reaches such an access by
asking the index about the byte before the cursor and keeping it only where it ends *at* the cursor; the
containment itself is untouched, because `selectionRange` is frozen on a cursor being outside the node it
touches.

**The `\` defect is still live and is stage 5's**, exactly as `docs/agent/loop-goal.md:30` says. Its check
`every_trigger_character_reaches_an_arm_that_is_not_the_position_list` moved from stage 0 to stage 5's
arms block, where its fix is: a catch-up failure returns before every other check in the sweep
(`tools/loop.py:2639`), so gating a stage-5 fix at stage 0 would have reported no other stage until it
landed. `capabilities.rs:174`'s trigger list is deliberately not trimmed.

Nothing is blocked. Stages 2, 3, 4, 5 and 6 are all open; stage 1 is goal `editor-install`'s floor.

## Next group

**Stage 2: the one workspace symbol index** — one file set: `crates/nvs-lsp/src/document.rs`,
`crates/nvs-lsp/src/lib.rs`, `crates/nvs-lsp/tests/latency.rs`. Read
`rule:ide/a-full-reanalysis-stays-under-a-bound` before choosing the invalidation shape: the bound has to
hold with the index warm, which is what rules out rebuilding it per keystroke.

- [ ] **One construction site, beside `Analysed`** at `crates/nvs-lsp/src/document.rs:254`, exported from
      the module list at `crates/nvs-lsp/src/lib.rs:96`. `rule:ide/five-features-are-one-reference-index`
      is the whole specification and the test is `the_crate_has_exactly_one_symbol_index_construction_site`.
- [ ] **Invalidate the file and its readers and nothing else.** `Documents` at
      `crates/nvs-lsp/src/document.rs:94` already holds the version per URI, so what changed is a fact it
      has; the test is `a_change_invalidates_the_file_and_its_readers_and_nothing_else`.
- [ ] **The warm bound**, copied from the guard shape already in the crate —
      `crates/nvs-lsp/tests/latency.rs:130` is `a_full_reanalysis_of_a_thousand_lines_stays_under_the_bound`
      and is `#[cfg(debug_assertions)]` for the reason the playbook's `cargo-named` bullet gives. Test:
      `a_warm_index_answers_within_the_reanalysis_bound`.
- [ ] **Scope selects the tree, never the construction site** —
      `rule:ide/check-scope-defaults-to-open-documents`, at the walk `crates/nvs-lsp/src/document.rs:381`.
      Test: `check_scope_selects_the_tree_and_never_the_construction_site`.

## Backlog

- The corpus owes 15 cells before `}`, `)` and `]` can each hold a case: `docs/agent/loop-goal.md:26`
  asks for one per closing delimiter, and `crates/nvs-lsp/src/coverage.rs` § *Decision* prices it.
- Stage 4 opens this goal's one record, next free 0171 — `docs/agent/loop-goal.md` § *Standing decisions*.
- Inherited members and visibility are still not applied to a member list —
  `crates/nvs-lsp/src/completion.rs` § *Known gaps*.
- `[context] modules` gained `crates/nvs-lsp/src/coverage.rs` this session; the pack never printed what a
  `.lspt` case costs, which is what made stage 0 wider than its item read.
