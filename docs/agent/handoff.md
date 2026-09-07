# Handoff

## State

**Goal 12 — the resilient tree — is complete.** Stages 2 to 7 are landed and green: explicit
recovery, the index, the prefix sweep, positions, the section lexer, and `nvs ast --json`. Stage 7's
three acceptance checks all pass — the four `-p nvs-cli` tests, the node shape off the binary, and
`python tools/reference.py --check`.

**`nvs ast --json` is the whole document `rule:ide/ast-json-schema-is-frozen` specifies.** It parses
through `nvs_syntax::parse` rather than `parse_file`, so the trivia comes back with the statements,
and a trivium is a node of the same shape — `Whitespace`, `LineComment`, `BlockComment` or
`DocComment`, its span, no children — sitting under the innermost node whose span contains it.
`crates/nvs-cli/src/ast.rs`'s module doc argues why all four kinds are there rather than comments
alone, and what that spends.

**The schema is frozen from two sides** in `crates/nvs-cli/tests/ast.rs`: the shape is asserted over
every `examples/**/*.nvs`, with `SCALARS` the closed roster of scalar field names a node may carry
beside `kind`, `span` and `children`, and `examples/hello.nvs`'s whole document is frozen byte for
byte. A seventh scalar reaching the JSON fails that test until the slice adding it says so.

**Still open under this goal, both in the Backlog below**: the `secret` placeholder, which waits for
a caller that has type-checked, and `ExprKind::ClassConstAccess`, which is the next group.

## Next group

**`::` recovers like every other member access**, over `crates/nvs-syntax/src/ast.rs`,
`crates/nvs-syntax/src/parser/expr.rs`, `crates/nvs-syntax/src/walk.rs` and
`crates/nvs-cli/tests/ast.rs`. The tree already says which form a `->` member name took; a `::` one
cannot say it at all.

- [ ] **`ClassConstAccess` carries a `MemberName`** — `crates/nvs-syntax/src/ast.rs:932` holds a
      bare name where `PropertyAccess` and the method accesses hold a `MemberName`, so a source
      ending in `Foo::` recovers into a node no consumer can tell from one somebody wrote.
      `rule:ide/recovery-is-explicit` — the production in
      `crates/nvs-syntax/src/parser/expr.rs` follows the field.
- [ ] **The walk says so, and the JSON shows it** — the arm calling
      `crates/nvs-syntax/src/walk.rs:664`'s `member_form` gains that production, which is what puts
      `"member": "missing"` on the node. `rule:ide/ast-json-schema-is-frozen`.
- [ ] **A fixture ends in `Foo::`** beside `crates/nvs-cli/tests/ast.rs:86`'s `recovered.nvs`, and
      `ast_json_includes_trivia_and_recovery_nodes` asserts the same pair for `::` as it does for
      `->`. `SCALARS` at `crates/nvs-cli/tests/ast.rs:214` already admits `member`, so nothing else
      moves.

## Backlog

- The `secret` placeholder in `nvs ast --json` waits for a caller that has type-checked — `rule:ide/ast-json-schema-is-frozen`.
- The `.lspt` runner is goals 14/15's, not this one — the goal's § *Standing decisions*.
- A BOM is left in the text and counted as one code unit; stripping it is the document store's — `rule:ide/positions-have-one-home`.
- `aux_path`'s containment argument is `.nvst`'s reserved names plus a shape check; `.lspt` will want the shape half — `crates/nvs-test/src/case.rs:@aux_path`.
- `Parsed.index` is built and dropped by `nvs ast`, which is one walk this command never reads — `crates/nvs-syntax/src/parser/mod.rs:771`.
