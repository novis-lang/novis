# Handoff

## State

Goal `core-ast-and-2-more` (dossier), 3 of its 9 items landed. `Core\Ast::parse`,
`Core\Ast\Node::children` and `Core\Ast\Node::column` each report `complete` from
`python tools/dossier.py --id '<feature>'`: description, examples, attack, figure and a
`covers:` marker on both sides.

One finding is recorded rather than fixed, as `rule:testing/a-failing-proof-is-fixed-or-recorded`
allows: `Core\Ast::parse` spends time in the square of a line's length, because a node's column is
counted from the start of its line once per node. It is `# Known gaps` item 1 in
`crates/nvs-stdlib/src/ast.rs`, owned by M12, and
`tests/hostile/core/Ast/parse/02-a-program-written-on-one-long-line.nvs` is the marked attack. The
fix belongs in `nvs-diagnostics`, which `rule:ide/positions-have-one-home` makes the only home for
position arithmetic, so it is that crate's public surface rather than this module's.

## Next group

The rest of `Core\Ast\Node`, which shares one file set with what just landed:
`crates/nvs-stdlib/src/ast.rs`, `docs/examples/core/Ast-Node/`, `tests/hostile/core/Ast-Node/` and
`benches/members/core/Ast-Node/`. `rule:testing/feature-proofs` is what each one owes, and the
landed `column` directory is the shape to follow. The examples of these four must each be a
*different* use from `column`'s three, which is the only part of the group that is not mechanical.

- [ ] **`Core\Ast\Node::kind`** — owes examples, hostile, perf, and a `covers:` marker on both
      sides. `crates/nvs-stdlib/src/ast.rs:192`
- [ ] **`Core\Ast\Node::nodes`** — the whole subtree, where `children` is one step.
      `crates/nvs-stdlib/src/ast.rs:210`
- [ ] **`Core\Ast\Node::line`** — a near-twin of `column`; its `.nvst` marker goes in
      `tests/conformance/core/core-ast-node-position-names-the-first-character-of-its-production.nvst`.
      `crates/nvs-stdlib/src/ast.rs:219`
- [ ] **`Core\Ast\Node::offset`** — bytes where `column` is characters.
      `crates/nvs-stdlib/src/ast.rs:237`

## Backlog

- `Core\Attributes::all` and `::get` are items 8 and 9 of this goal, in
  `crates/nvs-stdlib/src/attributes.rs` — a different file set, so a group of their own.
- The known gap above is the only open finding; `python tools/dossier.py --gaps` lists it.
