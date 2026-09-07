# Handoff

## State

**Goal 10 — stage 5 is whole: `Core\Task::all` binds from its argument's *type*, and the
written-`fn`-literal restriction is gone.**

- **`CoreTy::CallableShapeTo` is re-founded as `CoreTy::ShapeOfCallables`, not deleted**, and
  `docs/decisions/0136.md` § *Revisiting* is where that reports back. It is now an ordinary type
  in both halves: what it accepts is `crates/nvs-types/src/expr/assign.rs:200`'s assignability arm
  (a `Ty::Shape` whose every field is a callable) and what it binds is
  `crates/nvs-types/src/generics.rs:214`'s own walk over the argument's fields. Neither reads an
  expression. **Deleting it outright was tried and rejected**: it is also the parameter's *bound*,
  and with only `CoreTy::Var("S")` there `Core\Task::all(5)` type-checks — see the playbook bullet.
- **`bind_callable_shape`, `generics::callable_shape_var`, `E0773` and `E0774` are gone.** A field
  holding a `callable(): T` variable, a first-class callable, or a whole shape held in a variable
  all bind exactly as a written literal does; a field declaring bare `callable` answers `mixed` for
  itself alone. `rule:concurrency/an-all-field-answers-what-its-callable-declares` replaced
  `…/an-all-field-must-be-a-written-fn-literal` at the same slot, with 0072/0114/0136's `changes:`
  blocks re-pointed.
- **`Ty::ShapeOfCallables` is the one checker type that survives substitution** —
  `crates/nvs-types/src/generics.rs:352` is the arm that says so — so `nvs-ir` meets it, and
  `erase_checked_ty` (`crates/nvs-ir/src/lower/mod.rs:3067`) answers `Ty::Object` for it beside
  `Shape`.
- **Two acceptance names moved** in `docs/agent/loop-goal.toml` and its byte-identical goal copy:
  `no_registry_row_names_a_callable_binding_site_variant` → `only_task_all_names_a_shape_of_callables`,
  and the goal-6-era `a_task_all_field_holding_a_callable_variable_is_a_compile_error` →
  `task_all_no_longer_refuses_a_field_that_is_not_a_literal`.

Conformance is 1570: two cases retired for a restriction that no longer exists, one agreement case
written in their place.

## Next group

**Stage 6 — spending the proof: a statically proven call site stops paying the per-argument tag
check** — file set: `crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/lower/closure.rs`,
`crates/nvs-runtime/src/closure.rs`, `crates/nvs-codegen`, `examples/typed-callable.nvs`. Take them
in order; the fixture is the whole goal read end to end and is the driver's oldest red check.

- [ ] **A call through a written signature emits no `check_param_tags`, and a bare `callable` still
      does.** The sequence is `crates/nvs-runtime/src/closure.rs:543`, reached from
      `call_closure` (`crates/nvs-runtime/src/closure.rs:161`); the emission decision is at the
      lowering of the call in `crates/nvs-ir/src/lower/expr.rs`, which is where the callee's checked
      type is still in hand. `rule:types/callable-signature`; `-p nvs-codegen` owes
      `a_proven_callable_call_site_emits_no_param_tag_check` and
      `a_bare_callable_call_site_still_emits_the_param_tag_check`
      (`docs/agent/loop-goal.toml` stage 6 is the whole list).
- [ ] **Both closure metadata slots stay on every closure object.** `CLOSURE_ARITY_SLOT` and
      `CLOSURE_PARAM_TAGS_SLOT` are written at `lower_closure`
      (`crates/nvs-ir/src/lower/closure.rs:160`); a literal does not know which kind of site will
      call it, so what stage 6 removes is the work and never the metadata —
      `a_closure_object_still_carries_both_metadata_slots`.
- [ ] **`examples/typed-callable.nvs` is written and prints the four lines the goal names.**
      The `exact` check at `docs/agent/loop-goal.toml:4740` is the specification, verbatim, down to
      `Task::all answers a shape of its callbacks' return types`; the surface it exercises is
      `crates/nvs-stdlib/src/task.rs:131`'s rows. This is the acceptance check that has been red
      since session 0003.

## Backlog
- The valgrind leg is not optional for stage 6 — `docs/agent/loop-goal.md` § *Stage 6* item 3.
- `[context] modules` for this goal has no `nvs-codegen` or `nvs-runtime/src/closure.rs` pattern,
  so stage 6 opens without its own map — add both when you take it.
- The handoff's file set named `docs/spec/01-core-library.md`, which no longer exists; the `Core`
  reference is `docs/reference/core/<Class>.md` and `docs/reference/lang/`.
- `docs/agent/loop-goal.md` § *Stage 5* still says the variant is deleted; ADR 0136 § *Revisiting*
  is the current answer, and the goal prose is frozen input rather than a rule.
