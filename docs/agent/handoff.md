# Handoff

## State

**M4, item 16 is half closed and the refusal ceiling is 4.** The tree is at **868 conformance
plus 189 differential**. Nothing is blocked.

Item 16's three sites were never about named or spread arguments — both of those had landed
(`ResolvedCall::arg_slots` carries the mapping, `lower_variadic_tail` the tail). All three were the
`let CallArgs::List(list) = args else` arm, reachable only by ADR 0027's `(...)`. Two are now proven
unreachable and reworded as engine invariants, so `CEILING` in
`crates/nvs-ir/tests/refusals.rs` is **4**; the allowlist is still empty and may never grow.

What landed on the checker side: `Foo::bar(...)` and `$obj->method(...)` record
`ExprInfo::CallableRef` — the same `ResolvedCall` a call records, under a variant nothing can
mistake for one — and the two shapes naming no member are diagnostics, `E0740` for `new C(...)`
(new) and `E0732` for a `mixed` receiver (already there). ADR 0027 § 1 is the home of that rule;
`nvs_types::expr_table::ExprInfo::CallableRef`'s own doc is the home of what the record carries.

`nvs-ir` reads none of it yet, so every first-class callable still panics — with a message that now
names `CallableRef` and says the arm is missing.

## Next group

**Item 16's lowering half: turn a `CallableRef` into a closure value.** ADR 0027 § 1 and
[ADR 0031](../adr/0031-callable-is-the-only-closure-type.md) § 2 specify it; `lower_closure` is the
model to copy, since a closure literal already lowers to an object of a synthesized class with one
field per capture and a bound receiver is exactly one such field.
File set: `crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/lower/closure.rs`,
`crates/nvs-ir/src/lower/call.rs`, `crates/nvs-ir/tests/refusals.rs`.

- [ ] **`$g(...)` on a value already typed `callable` is the identity** — PHP hands back the same
      Closure object, so this is "lower the callee and return it", with the retain decision taken
      through `Lowering::aliasing_read` like every other producer. It is the one *counted* refusal
      item 16 still holds. `crates/nvs-ir/src/lower/call.rs:766` (`lower_closure_call`'s sentinel
      arm), `crates/nvs-ir/src/lower/closure.rs:149` (`lower_closure`, for the ownership shape).
- [ ] **Lower a static `CallableRef`** — `Class::method(...)`, `self::`/`static::`/`parent::`.
      No receiver to capture, so the synthesized class has no fields; `ResolvedCall::static_class`
      is what `static::` means and must ride into the value.
      `crates/nvs-ir/src/lower/expr.rs:2622` (the `let Some(ExprInfo::Call(call))` arm to precede).
- [ ] **Lower an instance `CallableRef`** — `$obj->method(...)`. One captured field, the receiver,
      retained at the reference and released with the closure; `ResolvedCall::overridden` decides
      whether the body binds a label or goes through `InstKind::CallVirtual`.
      `crates/nvs-ir/src/lower/expr.rs:2433`. Lower `CEILING` to 3 in the same slice if
      `call.rs:766` went with it.

## Backlog

- Item 25's two — `object` as a declared type has a representation arm.
  `crates/nvs-ir/src/lower/mod.rs:2701` and `:2784`; `docs/agent/loop-goal.md` item 25.
- Item 4's one — the bitwise operators. `docs/agent/loop-goal.md` item 4.
- A `Core` member referenced as `Core\Str::length(...)` has `has_body: false` and names no compiled
  function, so its closure body has to be an `InstKind::CoreCall` shim — worth deciding before the
  static arm is written. `nvs_stdlib::registry` owns what a `Core` member is.
- `check_and_find_expr_span` in `crates/nvs-types/src/expr_table.rs` only reads the first statement
  of `T::m` and only as a bare expression or `return`, so a fixture for anything on the left of a
  `=` has to be written as a `return`. Not a gap, just the shape.
