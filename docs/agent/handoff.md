# Handoff

## State

**Goal 10 — stage 3 is whole: a closure literal's parameter may leave its type out, and takes
it from the position the literal stands in.** Three layers, one rule
(`rule:types/callable-literal-inference`):

- **Parser.** `parse_closure_params` (`crates/nvs-syntax/src/parser/expr.rs:1670`) is the one
  list where a missing type is not reported; every declaration still goes through
  `parse_params` and keeps `rule:types/declaration`'s refusal.
- **Checker.** `check_fn_literal` (`crates/nvs-types/src/expr/calls.rs:1564`) takes the
  expected `TypeId`, reads `Ty::CallableSig`'s parameter list and fills each empty slot.
  `E0808` names a parameter that had nothing to take — bare `callable`, or a signature shorter
  than the literal — and it is checked as `mixed` so the body is still checked at all.
- **IR.** `closure_param_ty` (`crates/nvs-ir/src/lower/closure.rs`) reads that answer back: the
  checker records it under the parameter's own name span, because an inferred parameter has no
  annotation node for `lower_decl_type` to read. The playbook bullet above is the trap that made
  this third layer necessary.

`tests/conformance/lang/an-unannotated-closure-parameter-takes-its-type-from-the-position.nvst`
runs all of it end to end, a class-typed inferred parameter included.

**Stages 4-5 are what is left, and nothing reaches a real callback until they land.**
`Core\Arr::map($users, fn($u) => $u->name)` still refuses `$u` with `E0808`, because `map`'s
callback parameter is a `CoreTy::CallableTo` rather than a signature. The goal's acceptance
fixture `examples/typed-callable.nvs` is stage 6 and needs those two first.

The design is settled and is not re-derived: `rule:types/callable-signature`, `-arity`,
`-variance`, `-literal-inference`, plus the goal's own § *Standing decisions*.

## Next group

**Stage 4: a `Core` signature writes its callback's signature** — file set:
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/arr.rs`,
`crates/nvs-types/src/core_lib.rs`, `docs/spec/01-core-library.md`. Take both; the second is
where the first is proven, and the spec row moves in the same slice as the registry row.

- [ ] **A `Core` signature gains a variant that spells a written signature.** `CoreTy`
      (`crates/nvs-stdlib/src/registry.rs:406`) has `CallableTo` and `CallableShapeTo` and
      nothing that names parameter types; add the variant beside them and lower it at
      `crates/nvs-types/src/core_lib.rs:469` to `interner.callable_sig`, with the call's own
      type-variable substitution already applied — the expected type that reaches
      `check_fn_literal` has to be the substituted one or `T` never becomes `User`.
      `rule:types/callable-signature`.
- [ ] **`Core\Arr::map` and `Core\Arr::filter` carry theirs, spec row included.**
      `crates/nvs-stdlib/src/arr.rs:116` is `map`'s row. The result-variable binding
      `Ty::CallableTo` does today lives at `crates/nvs-types/src/expr/args.rs:1176` and is what
      the new variant replaces for these two members — a signature's own return type binds `U`,
      so the bespoke path is dead weight once the row moves. `docs/spec/01-core-library.md` § 2
      changes in the same slice, `rule:core-api/reference-card`.

## Backlog
- Stage 5: `Core\Task::all`'s `CoreTy::CallableShapeTo` (`crates/nvs-stdlib/src/registry.rs:432`)
  — ADR 0072 § 1.
- Stage 6: `examples/typed-callable.nvs`, the goal's four `want` lines in
  `docs/agent/loop-goal.toml`.
- Every `rule:types/callable-*` fragment still reads `status: designed` while three of them are
  implemented and guarded; `tools/rules.py` offers no way to flip one, so who does it is
  undecided — `docs/agent/doc-cleanup.md`.
- `check_param_tags` still runs per argument for a call through a written signature; discharging
  it at compile time is ADR 0136 § *In short*'s priority-1 half.
- `nvs-cli` and `nvs-stdlib` still describe a callback as `callable` in help text; harmless, but
  it stops reading as the truth once stage 4 lands — `docs/spec/01-core-library.md`.
