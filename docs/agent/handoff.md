# Handoff

## State

**Goal 10 — stage 4 has its two halves in the type layer, and no `Core` row carries one yet.**

- **`CoreTy::CallableSig(&[CoreTy], &CoreTy)`** (`crates/nvs-stdlib/src/registry.rs:433`) spells a
  written callback signature, and `nvs_types::core_lib`'s `lower` interns it field-wise to
  `Ty::CallableSig` — so a variable inside it is the same `Ty::TypeVar` the member's other
  parameters intern to, and `nvs_types::generics::substitute` (which already rewrote that type
  field-wise) carries the call's own bindings through it. `rule:types/callable-signature`.
- **`nvs_types::generics::bind` pairs two `Ty::CallableSig`s** — a declared
  `callable(T, string): U` against the signature a written `fn` literal reports binds both variables
  out of the one argument, structurally, where `Ty::CallableTo` needed a bespoke read of the
  literal's recorded return type. Parameters stop at the shorter list, which is
  `rule:types/callable-arity`'s prefix match at the binding pass.

**What blocks the rows is a third piece, and it is stage 5's.** `nvs_types::expr::args`'
`check_generic_args` checks a closure argument in its **first** pass, before `sig.substituted`, so a
row spelling `callable(T, string): U` today would reach `check_fn_literal` with no expected type at
all and `E0808` would refuse `fn($u) => …` exactly as it does now. The playbook bullet above is that
trap. `Core\Arr::map` and `Core\Arr::filter` still carry `CoreTy::Callable`/`CoreTy::CallableTo`,
and the goal's acceptance fixture `examples/typed-callable.nvs` is stage 6, behind both.

The design is settled and is not re-derived: `rule:types/callable-signature`, `-arity`, `-variance`,
`-literal-inference`, plus the goal's own § *Standing decisions*.

## Next group

**Stage 5: a closure argument is checked against the substituted parameter, and then the rows
land** — file set: `crates/nvs-types/src/expr/args.rs`, `crates/nvs-stdlib/src/arr.rs`,
`docs/spec/01-core-library.md`. The first item is what the second needs; take them in order.

- [ ] **A `fn` literal at a parameter that mentions a type variable is deferred to the substituted
      pass.** `check_generic_args` (`crates/nvs-types/src/expr/args.rs:1186`) checks every argument
      but the options bag at `crates/nvs-types/src/expr/args.rs:1225`, then substitutes at
      `crates/nvs-types/src/expr/args.rs:1286` and re-checks the bag at
      `crates/nvs-types/src/expr/args.rs:1293`. Defer a closure literal the same way — its own
      declared signature is still what `crate::generics::bind` reads at
      `crates/nvs-types/src/expr/args.rs:1277`, so the binding pass must keep seeing it while the
      *check* moves to the third pass. `rule:types/callable-literal-inference`.
- [ ] **`Core\Arr::map` and `Core\Arr::filter` carry `CoreTy::CallableSig`, spec row included.**
      `crates/nvs-stdlib/src/arr.rs:107` is `map`'s row and its callback is `CoreTy::CallableTo("U")`
      today; `filter`'s binds nothing and is the safer one to move first. The spec's § 2 row moves in
      the same slice — `rule:core-api/reference-card`, and the goal's § *Standing decisions* says so
      outright. `rule:types/callable-signature`.

## Backlog

- Stage 6: `examples/typed-callable.nvs`, the goal's acceptance fixture — `docs/agent/loop-goal.md`.
- `Ty::CallableTo`/`CallableShapeTo` are deleted once every row spells a signature — ADR 0136 § *In
  short*.
- `Core\Task::all`'s `CallableShapeTo` is the second binding site, and the last one — ADR 0072 § 1.
