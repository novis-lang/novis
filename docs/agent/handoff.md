# Handoff

## State

**M4 — language completeness.** Both catch-alls in `mwl-ir`'s *statement* slice are closed, and
the last handoff's method — enumerate the roster, do not trust the message — found four live holes
where the plan claimed none, so it is now a playbook bullet rather than a habit.

- **`lower_stmt`'s statement-kind catch-all** (`crates/mwl-ir/src/lower/stmt.rs`). `StmtKind` has
  31 variants against 18 arms; of the thirteen left, `global`/`goto`/function-scope `static` and
  `Error` die in the parser, `var $x;` is `E0101`, a top-level `function`/`const` is
  `E0215`/`E0216`, and the seven **declarations** are now uniform: skipped at file scope by
  `lower_script_stmts` (`autoload` was missing from that list, so an entry-point `autoload`
  panicked) and `E0233` from `mwl_types::locals::nested_declaration` anywhere else (`namespace`,
  `use`, `type` and `autoload` were silently accepted). `mwl_types::check::check_stmts` now matches
  `TypeAliasDecl`/`AutoloadDecl` at file scope, which is what makes "arriving is the test" true for
  all seven rather than three. The arm's own doc comment carries the subtraction.
- **`lower_expr_stmt`'s catch-all** (same file). It refuses nothing now: an expression used as its
  own statement is lowered for its effects and its value discarded, guarded by
  `Lowering::aliasing_read` so `$x;` does not release a reference this frame never took. Refusing
  the effect-free ones was rejected — "has no effect" is not decidable here (a property read runs
  its ADR 0014 § 1 hook), and priority 2 says a statement PHP evaluates evaluates. A shape with no
  lowering is named one level down by `lower_expr`'s own dispatch.
- **`$a = &$b;` is `E0701`** (`crates/mwl-types/src/expr/assign.rs`, reported from
  `expr/mod.rs`'s `Assign` arm so every position takes it). ADR 0031 § 2 removed by-reference
  capture and ADR 0023 makes the right-hand side a copy, so the `&` has no owner — the same
  reasoning `E0483` already applies to `[&$x]`. `&$x` at a *call* site is untouched.

`python tools/holes.py` attributes nothing to this group any more; its worklist is items 1, 4, 6,
16 and 25.

## Next group

**The `mwl-ir` *expression* dispatch's catch-alls** — the same "enumerate the arms, do not trust
the message" job, one file over. The file set: `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-syntax/src/ast.rs` (for the `ExprKind`/`UnaryOp`/`BinaryOp` rosters),
`tests/conformance/lang/`. Note that `lower_expr_stmt` now *routes* every unhandled statement-shape
here, so this dispatch's message is what a user sees for one — its wording is load-bearing now.

- [ ] **`expr.rs:322` — the expression-kind dispatch's catch-all** (`holes.py --item 6`).
      Subtract the arms from `ExprKind`'s roster the way `stmt.rs` was just done, then one scratch
      `.mwl` per survivor. Anchors: `crates/mwl-ir/src/lower/expr.rs:322`,
      `crates/mwl-syntax/src/ast.rs:@ExprKind`, `crates/mwl-ir/src/lower/expr.rs:44` (`lower_expr`).
- [ ] **`expr.rs:2679` — the unary-operator catch-all.** `UnaryOp` is a short roster; the arm
      claims only `-`/`!`/`~` lower. Anchors: `crates/mwl-ir/src/lower/expr.rs:2649`
      (`lower_unary`), `crates/mwl-syntax/src/ast.rs:@UnaryOp`.
- [ ] **`expr.rs:3126` — the binary-operator catch-all.** Same shape against `BinaryOp`, and the
      playbook's `emit_binop` bullets say which rows the checker accepts but codegen refuses —
      that gap is the likely residue. Anchors: `crates/mwl-ir/src/lower/expr.rs:2840`
      (`lower_binary`), `crates/mwl-syntax/src/ast.rs:@BinaryOp`.

## Backlog

- `crates/mwl-codegen/src/ty.rs:116` and `:121` — two refusal sites no `holes.py` item anchors.
- ADR 0007 § 2's `array<T> as array<U>` still panics `mwl-ir`; it blocks several case spellings
  (playbook, *Writing a test case*).
- `$c = require 'config.mwl';` — ADR 0021 § 3's value form, still a gap (`mwl-ir` crate docs).
- `mwl_types::locals` still does not descend into a nested declaration's *body*, so an error
  inside a refused `namespace { … }` block is not reported (that module's known gaps).
- `holes.py` items 1, 4, 16 and 25 — the promotion table, the bitwise operators, named/spread
  arguments, `object` as a declared type.
