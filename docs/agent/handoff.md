# Handoff

## State

**`Parsed` carries all three of `rule:ide/one-grammar-one-tree`'s fields: `stmts`, `trivia` and now
`index`.** `crates/nvs-syntax/src/index.rs` is the new module — `SyntaxIndex::at(offset)` answers a
`NodePath`, the innermost node containing the offset then its ancestors outward, which is
`rule:ide/the-index-answers-the-cursor` whole. `python tools/verify.py` is green.

- **The index is flattened from `walk.rs`'s walk, not from a second match on the grammar.** A
  `walk::Node` now carries its span, `walk::of_stmts` is the walk over statements already parsed, and
  `SyntaxIndex::of_stmts` flattens that into `(kind, span, parent)` rows in pre-order. So a production
  added to `ExprKind` or `StmtKind` still lands in exactly one `match`, and `Core\Ast` and the index
  share one vocabulary of kinds. The two decisions this took — that node set, and half-open
  containment, so a caret at a node's end is *between* nodes — are `index.rs`'s module doc.
- **`parse()` builds it; `parse_file()` is untouched**, so no compile path pays for it. The spend is
  named in the module doc: one row per node plus the intermediate `walk::Node` tree, both dropped per
  analysis.
- Nothing is blocked. `rule:ide/the-index-answers-the-cursor` stays `designed` like the rest of the
  `ide` chapter — goal 11 left its own rules `designed` too, so status flips with the milestone rather
  than with a slice.
- **A pack gap:** `[context] modules` in `docs/agent/loop-goal.toml` names no pattern for
  `crates/nvs-syntax/src/walk.rs`, which is where this item's work actually landed, and now misses
  `index.rs` as well. Add both, or widen the `nvs-syntax` pattern to the whole `src/`.

## Next group

**Recovery in `nvs-syntax`'s AST, and the bound that keeps a rebuild honest** — `ast.rs` and
`parser/expr.rs`, with the index tests beside them.

- [ ] **Recovery says so rather than being inferable** — `rule:ide/recovery-is-explicit`:
      `MemberName::Missing(Span)` beside `Ident`, and `ExprKind::Error` carrying what it stood in for.
      `crates/nvs-syntax/src/parser/expr.rs:969` is the *one* site that invents a member name today
      (`MemberName::Ident(self.error_expected("a member name"))`, an empty span a consumer cannot tell
      from a written one); `crates/nvs-syntax/src/ast.rs:386` is the enum. The in-crate matches that
      must be updated are `crates/nvs-syntax/src/walk.rs:589` and
      `crates/nvs-syntax/src/casing.rs:664`. `MemberName` is `#[non_exhaustive]`, so the twelve
      downstream sites in `nvs-hir`, `nvs-ir` and `nvs-types` compile unchanged through their wildcard
      arms — that is the risk, not the work: each has to *decide* about `Missing`, and today's
      empty-span `Ident` reaches them as a name, so a suppressed cascade is a diagnostic change the
      conformance corpus will see.
- [ ] **A rebuild of a ~1,000-line document stays under a named bound** —
      `rule:ide/a-full-reanalysis-stays-under-a-bound`, whose guard is a test rather than an
      assumption. `crates/nvs-syntax/src/index.rs:119` (`of_stmts`) is what is being measured, and its
      module doc names the intermediate tree as the thing to attack first if the bound fails.

## Backlog

- `Core\Ast\Node` exposes no span though `walk::Node` now carries one; whether it should is
  `rule:core-classes/ast-is-inert`'s call, in `crates/nvs-stdlib/src/ast.rs`.
- `SyntaxIndex` has no LSP consumer yet — hover, definition, completion and `selectionRange` are each
  a projection of `at()`, per `rule:ide/the-request-set-is-closed`.
- Carried gaps that must survive a goal switch: `docs/agent/carried-gaps.md`.
