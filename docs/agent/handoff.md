# Handoff

## State

**Goal 12 — the resilient tree — has stages 2 to 6 landed and green, and stage 7 half landed.**
`nvs ast` now takes `--json`, `--resilient` and `--strict`, in `crates/nvs-cli/src/ast.rs`, which is
the subcommand's own module rather than a `run_ast` in `main.rs`. Resilient is the default and
strict the opt-in, which is `ast_resilient_is_the_default_and_strict_is_opt_in`; the other three
tests the acceptance check names are unwritten, so the check is still red.

**A node carries the scalars its span does not show.** `walk::Node` gained `fields` and
`walk::Field` — an operator, a flag, and which form a member name took, so
`rule:ide/recovery-is-explicit`'s `Missing` is visible to a consumer that never sees a `MemberName`.
**A literal's text is deliberately not among them**, and `crates/nvs-syntax/src/walk.rs`'s third
decision is where that is argued: the span names it, and `nvs ast` does not type-check, so it cannot
know which literal is `secret` and owes the placeholder `rule:ide/ast-json-schema-is-frozen`
requires. Trivia is not in the JSON yet either — that is the next group's first item.

**The `nvs-server` check the driver reported red after session 0005 was load flake, not a
regression**: `a_connection_over_its_budget_is_closed_with_the_defined_code_not_oom` passes alone
and the whole `-p nvs-server` suite passes with it. The playbook bullet under *Tooling* is that.

## Next group

**Stage 7's remaining half**, over one file set: `crates/nvs-cli/src/ast.rs`,
`crates/nvs-cli/tests/ast.rs` and the fixture beside it. The reference chapter is already done, so
what is left is what the schema *contains* and the snapshot that freezes it.

- [ ] **Trivia and recovery nodes join the tree** — `document` at `crates/nvs-cli/src/ast.rs:82`
      builds from `walk::of_stmts`; it wants `nvs_syntax::parse` at
      `crates/nvs-syntax/src/parser/mod.rs:771` instead, whose `Parsed.trivia` is a `Vec<Trivia>`
      of `kind` and `span` (`crates/nvs-syntax/src/token.rs:43`), merged into the children in
      offset order. `rule:ide/ast-json-schema-is-frozen` — `ast_json_includes_trivia_and_recovery_nodes`.
- [ ] **The snapshot freezes the schema** — a golden over `examples/` in
      `crates/nvs-cli/tests/ast.rs:1`, beside the harness already there, plus the file that does not
      parse at `crates/nvs-cli/tests/fixtures/ast/recovered.nvs:1`.
      `rule:ide/ast-json-schema-is-frozen` — `ast_json_matches_its_frozen_schema` and
      `ast_json_renders_a_file_that_does_not_compile`.

## Backlog

- The `secret` placeholder in `nvs ast --json` waits for a caller that has type-checked — `rule:ide/ast-json-schema-is-frozen`.
- `ExprKind::ClassConstAccess` should carry a `MemberName` — `crates/nvs-syntax/src/ast.rs`, owed to `rule:ide/recovery-is-explicit`.
- The `.lspt` runner is goals 14/15's, not this one — the goal's § *Standing decisions*.
- A BOM is left in the text and counted as one code unit; stripping it is the document store's — `rule:ide/positions-have-one-home`.
- `aux_path`'s containment argument is `.nvst`'s reserved names plus a shape check; `.lspt` will want the shape half — `crates/nvs-test/src/case.rs:@aux_path`.
