# Handoff

## State

**M4 — language completeness.** An assignment in value position runs: `int $b = ($a = 2);` and
`$a = $b = 0;` answer the value **written**, at the target's declared representation, for every
target the statement form already had. The enabling change is that
`Lowering::lower_store` returns `(ValueId, Ty)` and takes an `extra_owner` flag
(`crates/mwl-ir/src/lower/stmt.rs:632`); `lower_read_modify_write` passes it through, so a compound
spelling in value position answers the value after the operation. `print` lowers in both positions
as well (`Lowering::lower_print`, `crates/mwl-ir/src/lower/expr.rs:345`).

**`exit` was filed as the cheap half of that item and is not.** There is no `process::exit` in the
tree and there must not be one in a helper — priority 1 makes termination request-scoped, so it
needs a distinguished unwind reaching `mwl-cli`'s exit status, across files this group never opened.
It is slice 3 below with its own file set.

`verify.py` 6 of 6 green — conformance **607**, differential **170**, 1632 unit tests.
`python tools/holes.py` reads **24 sites, 6 items**; both dispatch catch-alls stay one site each
whatever still reaches them.

## Next group

**`isset(...)` and `empty(...)`**, which are one pair of slices over one file set:
`crates/mwl-types/src/expr/mod.rs` with `crates/mwl-ir/src/lower/expr.rs` beside it. Take them in
this order — the second is the first one's lowering with ADR 0035's table in place of a null test.
The third has a different file set entirely and is sized accordingly.

- [ ] **`isset(...)` — the checker half first.** ADR 0028 § 3 fixes `isset($x)` as `$x != null`,
      and `isset($a, $b)` is the conjunction. The operands are **not checked at all** today —
      `ExprKind::Isset(_) | ExprKind::Empty(_) => env.interner.bool_ty()` at
      `crates/mwl-types/src/expr/mod.rs:448` never calls `check_expr` on them — so no `ExprInfo`
      is recorded and lowering one would panic on a property or an element before it reached any
      null test. An absent array key is the case that decides the shape: it must answer `false`
      rather than throw, which is `Env::coalesce_guarded`
      (`crates/mwl-types/src/expr/mod.rs:197` marks the levels, `:339` reads the mark) and
      `AbsentKey` (`crates/mwl-ir/src/ir.rs:1244`), both of which `??` already uses for exactly
      this. Mark the operand's subscript levels the same way and the lowering is a null test.
- [ ] **`empty(...)`** — the same operand handling with ADR 0035's truthy table negated instead of
      a null test: `Lowering::truthy_convert` (`crates/mwl-ir/src/lower/expr.rs:1094`) is the whole
      of the second half and already has a row for every representation but `Ty::Void`. Both
      operands reach `lower_expr`'s catch-all at `crates/mwl-ir/src/lower/expr.rs:292` today.
- [ ] **`exit` and `exit(...)`** — its own file set (`crates/mwl-runtime/src/abi.rs:53`'s `Fault`,
      `crates/mwl-ir/src/lower/stmt.rs:305`'s statement catch-all, `mwl-cli`'s exit status). Decide
      and record the termination path first: a `Fault` variant that unwinds to the top and sets the
      status keeps termination request-scoped, where a helper calling `process::exit` would not.
      PHP's two readings — `exit(int)` is the status, `exit(string)` writes and exits `0` — are
      selectable statically here, since ADR 0007 gives the operand a declared type.

## Backlog

- `$a = &$b` lowers in no position; it reaches `lower_expr_stmt`'s catch-all
  (`crates/mwl-ir/src/lower/stmt.rs:305`) and now `lower_expr`'s as well. No item names it.
- `$n + Adder::bump($n)` still reads the pre-call value — `pending_refs` is drained at the
  statement (`crates/mwl-ir/src/lower/mod.rs:1135`).
- `mwl-codegen/src/ty.rs:116` and `:121` are `holes.py`'s two unattributed sites.
- An enum case tagged into a `mixed` reads as its backing integer, so a `0`-backed case is falsy
  where ADR 0035 § 4 makes it truthy — `mwl_codegen::ty::tag_of`.
- `Core\Json::decodeAs<T>`'s wider codec-reachable set and its two default-bearing rows —
  `mwl_stdlib::json`'s own gaps.
- ADR 0088's qualifier classification — `mwl_stdlib::hash`'s module doc.
