# Handoff

## State

**Goal `fmt` (M10), stage 3 is three of its four names.**
`modifiers_are_written_in_the_canonical_order` landed: `nvs_syntax::Parsed` carries `modifiers` — one
entry per modifier list, in source order, each modifier with the span it was written at, collected in
`parse_modifiers` because every declaration's modifiers already go through that one loop, and never
collected on a compile path. `crates/nvs-fmt/src/modifiers.rs` rewrites a list as a permutation of
itself, so the spacing, the line breaks and any comment between two modifiers stay put.

**Only `a_declaration_brace_is_allman_and_a_control_brace_is_k_and_r` is left in stage 3**, and it is
the first rule that cannot land alone: 95 corpus `.nvs` files write a declaration's brace K&R.

**The corpus is still a fixed point of `nvs fmt`** (`crates/nvs-fmt/tests/identity.rs:36`), which is
stage 2's floor. Nothing is blocked.

## Next group

**Stage 3: the brace rule, and the corpus it moves** — one file set: `crates/nvs-fmt/src/print.rs`,
`crates/nvs-fmt/tests/base_style.rs`, `crates/nvs-fmt/tests/identity.rs`, and every `.nvs` under
`examples/` and `tests/`. The two slices are one decision: from the moment the rule lands until the
corpus is reformatted with it, stage 2's floor is red, so land them in one session.

- [ ] **The brace rule, and its case beside the other two** — `rule:tooling/fmt-base-style-is-per`. A
      declaration's `{` starts its own line at the declaration's own indentation; a control
      structure's stays on the keyword's line after one space, and `elseif`/`else`/`catch`/`finally`
      continue on the closing brace's line. Both are whitespace on either side of a brace, so this is
      `crates/nvs-fmt/src/print.rs:88`'s trivium walk rather than a keyword move —
      `crates/nvs-fmt/src/modifiers.rs:28` is the shape to reuse only where a byte really has to
      move. The case goes at `crates/nvs-fmt/tests/base_style.rs:83`, mangled about braces and about
      nothing else.
- [ ] **Reformat the corpus and re-green the floor** — `nvs fmt` over every `.nvs` under `examples/`
      and `tests/`, then `crates/nvs-fmt/tests/identity.rs:36` passes again. `examples/collect.nvs:2`
      is the line that moves in nearly every file, and the reformat shifts example line numbers,
      which the playbook records some `exact` checks as pinning.

## Backlog

- Stage 4's token rules — spacing, quotes, trailing commas — `docs/agent/loop-goal.toml` stage 4.
- PER's blank lines between members are still the author's — `crates/nvs-fmt/src/lib.rs` § *Known gaps* 1.
- A `switch`, a `match` arm list and a template region are still unplaced — same doc, gap 2.
- `private (set)` written across a skipped run leaves its whole list in the author's order — `crates/nvs-fmt/src/modifiers.rs:93`.
- A plain visibility written beside `private(set)` keeps the author's order between those two; the rule bands them — `rule:tooling/fmt-base-style-is-per`.
