# Handoff

## State

**Goal `fmt` (M10), half of stage 3 has landed.** `crates/nvs-fmt/src/indent.rs` places a line from
the innermost brace-delimited body on `SyntaxIndex::at`'s path and measures it from the line that
body's opener was written on; a line the tree does not place keeps the author's own whitespace. Two of
stage 3's four acceptance names are on disk — `indentation_is_four_spaces_per_block_depth` and
`an_authors_line_break_inside_an_expression_is_kept` — so that check stays red until the other two land.

**The corpus is still a fixed point of `nvs fmt`**, which is stage 2's floor
(`crates/nvs-fmt/tests/identity.rs`) and the thing every later layout rule has to keep true.

**The rustdoc gate is green again**: `[`format`]` in `crates/nvs-fmt/src/lib.rs` was ambiguous with
`std::format!` and now reads `[`format()`]`. Nothing is blocked.

## Next group

**Stage 3: the two base-style names that rewrite code rather than whitespace** — one file set:
`crates/nvs-syntax/src/ast.rs`, `crates/nvs-syntax/src/parser/`, `crates/nvs-fmt/src/print.rs` and
`crates/nvs-fmt/tests/base_style.rs`. Both are departures from "a code run is copied byte for byte",
which is `crates/nvs-fmt/src/print.rs:13`'s claim and has to be rewritten with them.

- [ ] **`modifiers_are_written_in_the_canonical_order`** — `rule:tooling/fmt-base-style-is-per`.
      `Modifier` carries no span, so the printer cannot find where one was written:
      `crates/nvs-syntax/src/ast.rs:516` is the enum, and the goal's § *Standing decisions* sends a
      missing piece of tree to `nvs-syntax` with its crate module doc recording it. Then the run
      between two trivia is rewritten in `crates/nvs-fmt/src/print.rs:38`.
- [ ] **`a_declaration_brace_is_allman_and_a_control_brace_is_k_and_r`** — same rule. The corpus
      writes a declaration's brace K&R (`examples/collect.nvs:2`), so Allman moves a line in nearly
      every example and the stage-2 floor fails until the corpus is reformatted with it; that
      reformat shifts example line numbers, which the playbook records some `exact` checks as pinning.
      Decide and land the two together.

## Backlog

- A property hook's body indents one level short — `crates/nvs-fmt/src/indent.rs`'s `BODIES` counts
  `Property` once where the property and the hook are two bodies.
- `switch`, `match` and template regions keep the author's indentation — `crates/nvs-fmt/src/lib.rs`
  § *Known gaps* 2 owns why, and stages 4 and 5 are where they land.
- Stage 4's token rules and stage 5's Novis constructs — `docs/agent/loop-goal.md`.
- Stage 6 wires `nvs fmt` as a subcommand with `tests/fmt/input` ↔ `tests/fmt/formatted` pairs —
  `docs/agent/loop-goal.md`.
