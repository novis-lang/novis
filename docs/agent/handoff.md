# Handoff

## State

**M4's Stage 6 is the frontier, and a method call now refuses every receiver
whose type names no class.** `E0477` is one code across that whole family —
`object`, an ADR 0036 shape, a union naming no single class, and the types that
can hold no object at all (a scalar, an `array<T>`, a `void` call's result) —
with only the help splitting three ways. `docs/adr/README.md`'s own paragraph
owns why the call half does not split into a deferral the way the property half
does, and `E_METHOD_ON_ERASED_RECEIVER`'s doc comment carries the roster.

- **`mixed` is the one receiver left, and it still panics** at
  `crates/nvs-ir/src/lower/expr.rs:2389`, whose roster now names it alone. That
  panic is the next group's whole subject.
- **The hard part of that group is not the dispatch but the marshalling.**
  `ClassDesc` (`crates/nvs-runtime/src/object.rs:206`) carries a name-keyed
  method table and the two derived fields `renderer`/`unwind`, but **no
  per-method signature** — a compiled method's parameters and return are its
  own representations, so a site holding only a tagged receiver has nothing to
  marshal against. Decide that first, in the session that needs it, under the
  goal's standing decisions.
- M4's acceptance still names *Verification* sections for ADRs 0023, 0028 and
  0069; 0014 and 0046 have theirs.

## Next group

**The `mixed` receiver `nvs-ir` still panics on.** One file set:
`crates/nvs-ir/src/lower/expr.rs:2389` (`lower_method_call`),
`crates/nvs-runtime/src/object.rs:206` (`ClassDesc`) and `:832`
(`ClassTable::set_methods`), `crates/nvs-runtime/src/closure.rs:132`
(`call_closure`, the precedent for calling a compiled function through a tagged
argument list), and `crates/nvs-types/src/expr/calls.rs:88` (the one site that
exempts `Ty::Mixed` from the refusal above).

- [ ] **Decide how a call through a `mixed` receiver marshals its arguments and
      its result, and record it** — ADR 0036 § 4, which grants the deferral and
      says nothing about the convention. The two shapes on the table are a
      per-method tag list on the descriptor (the `field_tags`/closure-param-tag
      precedent, `object.rs:206`) and a tagged-ABI thunk per method. Take it
      under the goal's standing decisions — a paragraph in
      `docs/adr/README.md` § *Decisions taken at project start*, not a new ADR.
- [ ] **Lower it** — one helper reached from `lower_method_call`
      (`lower/expr.rs:2389`), answering `Ty::Tagged`, with ADR 0036 § 4's two
      throws: the receiver whose tag is not an object, and the class that
      declares no such method. Ownership follows `call_closure`'s.
- [ ] **The case both halves owe** — the deferral's own `.nvst` beside
      `tests/conformance/reject/a-method-call-through-a-receiver-that-names-no-class-is-refused.nvst`,
      counting the agreement between a `mixed` receiver and the declared-type
      spelling of the same call rather than reading it off a line.

## Backlog

- A `require` whose path is not a string literal runs nothing at all, silently,
  in both forms — `nvs_hir::requires`' own known gap.
- An intersection type is unusable: `HasA&HasB $x = new Both();` is `E0401` at
  the binding, so no program reaches a method call through one (`nvs_types`).
- ADR 0033 § 4's container axis — a `secret` array element or shape-literal
  field carries no bit (`nvs_stdlib::debug`'s known gap 1).
- M4's acceptance owes *Verification* sections for ADRs 0023, 0028 and 0069.
