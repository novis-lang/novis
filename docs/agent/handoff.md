# Handoff

## State

**M4 — language completeness.** `isset(...)` runs end to end. ADR 0028 § 3's `!= null` over a
local, a `mixed`, a property, an element, a chain and a `?array<T>` no test narrowed, with a
list of operands as a left-to-right short-circuiting conjunction, in both value and statement
position. The checker half is the new `mwl_types::expr::presence`
(`crates/mwl-types/src/expr/presence.rs:44`), which marks every `Index` level of every operand
in `Env::coalesce_guarded` — the same set `??` fills — so an absent key answers `false`
instead of taking ADR 0007 § 7 row 11's throw. The lowering is
`Lowering::lower_isset`/`lower_isset_operand` (`crates/mwl-ir/src/lower/expr.rs:1573`, `:1640`).
An operand that names no storage is **E0498**.

**`empty(...)` is untouched and is the next slice.** `ExprKind::Empty(_)` still answers `bool`
with its operand unchecked, at `crates/mwl-types/src/expr/mod.rs:459`; it is `isset`'s operand
handling with ADR 0035's truthy table negated in place of the null test.

`verify.py` 6 of 6 green — conformance **609**, differential **171**, 1632 unit tests.
`python tools/holes.py` reads **24 sites, 6 items**; both dispatch catch-alls stay one site
each whatever still reaches them.

## Next group

**`empty(...)` first, then `exit`.** The first shares this session's whole file set —
`crates/mwl-types/src/expr/presence.rs` with `crates/mwl-types/src/expr/mod.rs` and
`crates/mwl-ir/src/lower/expr.rs` — and is the cheapest thing on the frontier because both
halves already exist one function away. The other two have file sets of their own.

- [ ] **`empty(...)`** — ADR 0035 § 2's truthy table, negated. PHP's `empty($x)` is `!$x` and
      accepts *any* expression (unlike `isset`, since PHP 5.5), so the E0498 shape check does
      **not** apply and only the guarded-subscript half carries over: factor the marking loop
      out of `check_isset_operand` (`crates/mwl-types/src/expr/presence.rs:44`) and give
      `ExprKind::Empty(_)` (`crates/mwl-types/src/expr/mod.rs:459`) its own arm calling it plus
      `check_expr`. The lowering is `Lowering::truthy_value`
      (`crates/mwl-ir/src/lower/expr.rs:1180`) then `UnOp::Not`, which is exactly `lower_not`
      (`:1673`) — so the arm may be one line beside the `Isset` one at `:291`, plus the
      statement-position twin at `crates/mwl-ir/src/lower/stmt.rs:225`. Watch the release: an
      operand that is a fresh producer owes one, and `truthy_value` already takes the
      `aliasing_read` answer as a parameter.
- [ ] **`exit` and `exit(...)`** — its own file set (`crates/mwl-runtime/src/abi.rs:53`'s
      `Fault`, `mwl-cli`'s exit status). There is no `process::exit` in this tree and there must
      not be one in a helper: priority 1 makes termination request-scoped, so under `mwl serve`
      at M7 a helper ending the process ends every other in-flight request with it. It wants a
      distinguished unwind carried to `mwl-cli`'s exit status.
- [ ] **A static property lowers nowhere, read or write** — the read panics `lower_expr`'s
      dispatch catch-all (`crates/mwl-ir/src/lower/expr.rs:291`) and the write
      `lower_stmt`'s reassignment arm (`crates/mwl-ir/src/lower/stmt.rs:1065`). The checker
      accepts both. No worklist item names it, so it is a hole `holes.py` does not count.

## Backlog

- An enum case tagged into a `mixed` reads as its backing integer, so a case backed by `0` is
  falsy where ADR 0035 § 4 makes every statically-typed case truthy — `mwl_codegen::ty::tag_of`.
- `Class::method(...)`, the first-class callable spelling, panics `mwl-ir` — gap 1.
- `array<T> as array<U>` does not lower (`crates/mwl-ir/src/lower/expr.rs:877`) — ADR 0007 § 2.
- `Core\Json::decodeAs<T>`'s wider codec-reachable set and its two default-bearing rows —
  `mwl_stdlib::json` gaps.
- M4S Part I depth: conformance 609 of 750, `python tools/gaps.py` ranks the thin classes.
