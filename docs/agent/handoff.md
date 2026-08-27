# Handoff

## State

**M4 — language completeness.** Both write-path panics the last handoff named are closed, and
neither by adding a lowering: each has **no reachable target left**, and each carries its own
proof in its own doc comment rather than in a doc that can drift away from it.

- **`Lowering::write_back_array`'s property arm** (`crates/mwl-ir/src/lower/mod.rs:1999`). A
  `PropertyAccess` span carries exactly one of `Property`, `HookedProperty`, `ShapeProperty`
  or nothing; `check_property_member` reports a diagnostic on every path it records nothing
  on, and `check_write_target` refuses the middle two as `E0478`/`E0480`, leaving `Property`.
- **`Lowering::row_ty_of`** (`crates/mwl-ir/src/lower/stmt.rs`), both halves. No entry means
  `mwl_types::expr`'s `ExprKind::Index` arm declined to record one, and every such path
  reports `E0482` or leans on a diagnostic already coming. A non-array element type cannot
  arise because *intermediate* means the level above resolved with this one as its base;
  `??`-guarded typing is the one exception and `Env::coalesce_guarded` is filled only from a
  `??`'s left operand, never from a write target — `??=` included.

`unset()` was the one write spelling that did not mark its subscript levels, so
`unset($erased->rows["0"])` took `E0482` for the subscript *and* `E0480` for the holder. It
marks them now (`crates/mwl-types/src/expr/members.rs`, the `Index` arm of
`check_unset_target`), and all four spellings now take exactly one diagnostic on the same
target — which is what the new agreement case asserts.

`python tools/holes.py` no longer attributes any site to this group; its worklist is now items
1, 4, 6, 16 and 25.

## Next group

**The `mwl-ir` lowering dispatch's own catch-alls** — the five `got {other:?}` refusals that
name a *kind* rather than a feature, which is the same "enumerate the arms, do not trust the
message" job the last two were. The file set:
`crates/mwl-ir/src/lower/expr.rs`, `crates/mwl-ir/src/lower/stmt.rs`,
`crates/mwl-syntax/src/ast.rs` (for the `StmtKind`/`ExprKind`/`UnaryOp`/`BinaryOp` rosters),
`tests/conformance/lang/`.

- [ ] **`stmt.rs:233` — the statement-kind dispatch's catch-all.** Enumerate `StmtKind`
      against the arms above it; the plan's *Open now* claims the dispatch has no shape left
      the checker accepts, so this is a proof to write down or a hole to find. Anchors:
      `crates/mwl-ir/src/lower/stmt.rs:233`, `crates/mwl-syntax/src/ast.rs:@StmtKind`.
- [ ] **`stmt.rs:364` — the expression-statement/reassignment catch-all.** Same file, same
      roster, one level in: which `ExprKind`s reach a bare expression statement. Anchors:
      `crates/mwl-ir/src/lower/stmt.rs:364`.
- [ ] **`expr.rs:317` — the expression-kind dispatch's catch-all** (`holes.py --item 6`).
      Enumerate `ExprKind` against `lower_expr`'s arms. Anchors:
      `crates/mwl-ir/src/lower/expr.rs:317`, `crates/mwl-ir/src/lower/expr.rs:44`.
- [ ] **`expr.rs:2679` and `expr.rs:3125` — the unary and binary operator catch-alls**, one
      slice because they are twins: `UnaryOp` and `BinaryOp` are both closed rosters and ADR
      0007 § 4's table says which rows exist. Watch the playbook's `emit_binop` bullets — a
      row the checker accepts is not a row that runs. Anchors:
      `crates/mwl-ir/src/lower/expr.rs:2679`, `crates/mwl-ir/src/lower/expr.rs:3125`,
      `crates/mwl-types/src/expr/operators.rs:403`.

## Backlog

- `mwl-ir` gap 21 (`crates/mwl-ir/src/lib.rs:535`) reads stale: `object $o = $obj;` lowers and
  runs today, so the gap text and its "one line arm" plan need re-checking or deleting.
- `crates/mwl-ir/src/lower/expr.rs:3441` — the instance-call panic still names "a `mixed`, a
  union or a scalar receiver, which the checker does not yet refuse". `E0235` closed only its
  computed-name half.
- ADR 0007 § 2's `array<T> as array<U>` conversion row still panics `mwl-ir`
  (`lower/expr.rs:877`), which is what keeps several `Core` refusals unreachable from source —
  `docs/agent/playbook.md` § *Writing a test case* carries the worked cases.
- `crates/mwl-codegen/src/ty.rs:116` and `:121` are the two refusal sites `holes.py` attributes
  to no item at all.
