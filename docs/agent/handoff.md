# Handoff

## State

**Goal `fmt` (M10), stage 2 has landed.** `crates/nvs-fmt/` is a workspace member and
`nvs_fmt::format` is its one entry: a `SourceFile` in, that file's canonical text out, or a `Refusal`
naming the file and carrying its parse's diagnostics. The printer is the identity — it reproduces every
`.nvs` file in the corpus byte for byte — and stage 2's two acceptance names are pinned in
`crates/nvs-fmt/tests/identity.rs`.

**The style is decided and landed.** [ADR 0039](../decisions/0039.md) and the fragments under
`docs/rules/tooling/fmt-*` are the spec, and [ADR 0173](../decisions/0173.md) added the `?>` bullet to
`rule:tooling/fmt-novis-constructs`. No session writes a record for this goal.

**Not wired yet, and not meant to be until stage 6:** `nvs fmt` is not a subcommand, `nvs-cli` does not
name `nvs-fmt`, and there are no `tests/fmt/input/` ↔ `tests/fmt/formatted/` fixture pairs. Nothing is
blocked.

## Next group

**Stage 3: the base style, PER wherever the grammar matches PHP's** — one file set:
`crates/nvs-fmt/src/print.rs`, `crates/nvs-fmt/src/lib.rs` and `crates/nvs-fmt/tests/`. Every rule
below is written into the run walk at `crates/nvs-fmt/src/print.rs:26`, which today copies each run
back unchanged. The four names are stage 3's `cargo-named` check verbatim.

- [ ] **`indentation_is_four_spaces_per_block_depth`** — `rule:tooling/fmt-base-style-is-per`. The
      whitespace trivium that opens a line is rewritten from the depth of the `Block` containing it;
      `crates/nvs-fmt/src/print.rs:26` is the walk, `crates/nvs-syntax/src/ast.rs:1097` is `Block`.
- [ ] **`a_declaration_brace_is_allman_and_a_control_brace_is_k_and_r`** — same rule: the run before a
      `{` is what carries the decision, at `crates/nvs-fmt/src/print.rs:26`.
- [ ] **`modifiers_are_written_in_the_canonical_order`** — same rule, and the one item that reorders
      tokens rather than whitespace: `crates/nvs-syntax/src/ast.rs:516` is `Modifier`.
- [ ] **`an_authors_line_break_inside_an_expression_is_kept`** — `rule:tooling/fmt-never-reflows`: a
      newline inside a whitespace run is the author's, and only the indentation after it is this
      tool's. Guards the three above, at `crates/nvs-fmt/src/print.rs:26`.

## Backlog

- Token boundaries: `Parsed` carries none, and inserting a space where the source has none needs them —
  decide it when stage 3 or 4 first requires it (`crates/nvs-fmt/src/print.rs` module doc).
- `nvs fmt` as a subcommand, with `--check`, `--diff` and `--stdin` — stage 6, `crates/nvs-cli/src/main.rs:193`.
- The frozen fixture pairs under `tests/fmt/` — goal `fmt` § *Standing decisions*.
- `nvs-fmt` has no `[context] modules` pattern yet; `tools/context-sync.py` sweeps it from this
  session's commits.
