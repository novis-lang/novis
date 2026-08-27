# Handoff

## State

**M4 — item 16 is closed at both ends: a `name:` argument and a `...` argument
lower at a resolved call, at a `new`, and through a `callable`.**
`mwl-ir`'s known gap 8 is gone from that list rather than reworded, and
`examples/callable.mwl` now prints six of its seven acceptance lines —
`byref=7 then 5` is item 18's and the only red one left in stage 3.

- **The resolved half needed nothing but its tests.** `lower_call_args` already
  placed each argument at the parameter its `ArgSlot` named
  (`crates/mwl-ir/src/lower/call.rs:74`) and materialized every unfilled
  parameter's default; what the tree owed was the evidence, which is now three
  snapshots and four checker tests. The panic the item pointed at
  (`call.rs:85`) refuses `CallArgs::FirstClassCallable` and nothing else —
  that is `mwl-ir` gap 1, not this item.
- **Through a `callable` the two halves part company, and ADR 0031 § 1 is why.**
  A `name:` has no parameter to fill at *either* end — a closure value records
  its arity and parameter tags, never names — so it is `E0712` where it is
  written (`mwl_types::expr::calls::report_named_args_through_callable`). A
  `...` needs no parameter list at all, so it lowers: the whole list becomes one
  array (`Lowering::lower_args_as_array`, shared with the variadic tail) behind
  `Helper::CallClosureArray`.
- **That is a second helper on purpose.** `CallClosure`'s argument count is a
  literal `mwl-codegen` writes beside the argument slot, and a spread's count is
  the one fact that is not known there. Both reach
  `mwl_runtime::call_closure` through one shared arity check
  (`call_closure_from_mwl`), so too few arguments is one catchable
  `LogicError` and not two. Valgrind-clean over a fixture that spreads a
  borrowed and a freshly built list two hundred times each.

## Next group

**Item 18 / `mwl-ir` gap 10: a `&$x` argument's copy-back, and the three stage-3
tests still owed.** The file set: `crates/mwl-ir/src/lower/mod.rs`,
`crates/mwl-ir/src/lower/call.rs`, `crates/mwl-ir/src/lower/stmt.rs`,
`examples/callable.mwl`.

- [ ] **A `&$x` argument's copy-back lands where the call is**, not at the
      enclosing statement — so such a call lowers in any expression position and
      a second read in the *same* statement sees the written-back value.
      `crates/mwl-ir/src/lower/mod.rs:1205` (`pending_refs`), `:2082`
      (`flush_ref_writebacks`), `crates/mwl-ir/src/lower/call.rs:846`
      (`stage_ref_arg`) and `:905`. `examples/callable.mwl` prints
      `byref=7 then 5` and the acceptance check wants `byref=7 then 7`, which is
      exactly this.
- [ ] **`a_reference_argument_lowers_in_any_expression_position`** — the
      `mwl-ir (calls)` acceptance list names it and the crate does not have it.
      A snapshot beside the three added this session
      (`crates/mwl-ir/src/lower/mod.rs:5599` onward).
- [ ] **`a_foreach_by_reference_writes_through_to_its_array` and
      `a_spread_array_element_lowers`** — the other two that list owes. Both
      behaviours already run (`examples/callable.mwl`'s `scaled=60` and
      `elements=4`), so these are tests over landed work, not new lowering.

## Backlog

- First-class callable syntax (`Class::method(...)`) still panics — `mwl-ir` gap
  1, at `lower/call.rs:85` and `:652`. Both are `CallArgs::FirstClassCallable`.
- Item 14's two temporaries shapes — `mwl-ir`'s own `Lowering` field docs.
- A call through a `callable` answers `mixed`, so a narrower position needs an
  `as T`; `examples/callable.mwl:4` is the worked spelling. ADR 0031 § 1.
