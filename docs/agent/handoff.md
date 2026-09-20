# Handoff

## State

Goal `core-ast-and-2-more` (dossier), 7 of its 9 items landed. Every `Core\Ast` and
`Core\Ast\Node` feature now reports `complete` from `python tools/dossier.py --id '<feature>'`:
`Core\Ast::parse`, and the node's `children`, `column`, `kind`, `nodes`, `line` and `offset`.
Only `Core\Attributes::all` and `Core\Attributes::get` are left, and they sit in a different
file set.

The Rust-side cases for `kind`, `nodes`, `line` and `offset` all live in
`crates/nvs-stdlib/src/ast.rs`'s `mod tests`, so they landed in one commit instead of one per
slice. Three conformance cases were re-attributed rather than duplicated: the column case
already asserts `line` and `offset`, and the transitive-closure case already asserts `nodes`,
so each gained a `covers:` marker (`rule:testing/proof-attribution`).

One finding is still recorded rather than fixed, as
`rule:testing/a-failing-proof-is-fixed-or-recorded` allows: `Core\Ast::parse` spends time in the
square of a line's length. It is `# Known gaps` item 1 in `crates/nvs-stdlib/src/ast.rs`, owned
by M12, and `tests/hostile/core/Ast/parse/02-a-program-written-on-one-long-line.nvs` is the
marked attack.

## Next group

The goal's last file set, and the only one left in it:
`crates/nvs-stdlib/src/attributes.rs`, `docs/examples/core/Attributes/`,
`tests/hostile/core/Attributes/` and `benches/members/core/Attributes/`.
`rule:testing/feature-proofs` is what each owes and the landed `Ast-Node` directories are the
shape to follow. These two are not shaped like a node walk: `Core\Attributes` is the one class
whose members never run, so read that module's own doc before pricing the bench and the attack.
The orientation pack prints neither `benches/members/README.md` § *What a bench declares* nor
`docs/examples/README.md` § *The description*, and both are needed to write a bench's declared
counts and an `about.md` — peek them once rather than guessing.

- [ ] **`Core\Attributes::all`** — owes `about.md`, examples, hostile, perf, and a `covers:`
      marker on both sides. `crates/nvs-stdlib/src/attributes.rs:65`
- [ ] **`Core\Attributes::get`** — the one-attribute half of the same retrieval; its examples
      must each be a different use from `all`'s three.
      `crates/nvs-stdlib/src/attributes.rs:56`

## Backlog

- `Core\Ast::parse`'s quadratic column cost — `crates/nvs-stdlib/src/ast.rs` `# Known gaps` 1, owner M12.
- goal `plain-comments` sweeps the landed proof comments and makes `dossier.py --comments` a gate.
- goal `the-description-is-owed` switches the `about.md` check on; until then only the prose asks for it.
