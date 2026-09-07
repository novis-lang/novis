# Handoff

## State

**Goal 10 — stage 4 is whole: every `Core` callback *parameter* writes its signature, and
`CoreTy::CallableTo` is deleted.**

- **Nineteen rows now spell what they hand their callback**, across `Core\Arr` (`mapKeys`,
  `groupBy`, `reduce`, `find`, `findKey`, `any`, `all`), `Core\Cli` (`live`, `progress`),
  `Core\Db\Connection::transaction`, `Core\Fatal`'s two, `Core\Heap`'s comparator,
  `Core\Out::capture`, `Core\Regex::replaceWith`, `Core\Script::onExit`, `Core\Task`'s `map` and
  `afterResponse`, and `Core\Test`'s three. `rule:types/callable-signature`. The spec's rows and
  `rule:tooling/in-place-output-is-a-scoped-live-region` were edited in the same slice, which is
  `rule:core-api/reference-card`.
- **A key-answering callback returns `int|string`, not `string`** —
  `crates/nvs-stdlib/src/arr.rs:128`'s `mapKeys` and `groupBy` take `ARRAY_KEY_CONTAGIOUS`, because
  `rule:types/arrays` normalises an integer key at the subscript rather than refusing it.
- **Bare `CoreTy::Callable` survives in exactly two places**, and
  `every_callback_parameter_declares_its_signature`
  (`crates/nvs-stdlib/src/registry.rs:3398`) is the list: `Core\Attributes::get`/`::all`, whose
  `$target` is a reference to a declaration and not a callback, and every **option bag's** field,
  which is not a parameter. That bag is now the only surface reaching
  `nvs_runtime::call_closure`'s per-argument tag check, so the six conformance cases that pin the
  runtime check ask it through `{by:}` instead of through `Core\Arr::any`/`::mapKeys`.
- **`Ty::CallableTo`, `TypeInterner::callable_to` and `generics::callback_result_var` are gone**
  with it; `Ty::CallableShapeTo` is the one binding site left, and stage 5 takes it.

Conformance is 1571 green.

## Next group

**Stage 5 — `Task::all`, and the second binding-site variant retired** — file set:
`crates/nvs-types/src/generics.rs`, `crates/nvs-types/src/expr/args.rs`,
`crates/nvs-stdlib/src/task.rs`, `crates/nvs-stdlib/src/registry.rs`, `crates/nvs-types/src/ty.rs`,
`docs/spec/01-core-library.md`. Take them in order; the second is the first one's fallout.

- [ ] **A shape of callables rebuilds a shape, and `CoreTy::CallableShapeTo` is deleted.**
      The variant is `crates/nvs-stdlib/src/registry.rs:427` and its `Ty` counterpart
      `crates/nvs-types/src/ty.rs:220`; its one row is
      `crates/nvs-stdlib/src/task.rs:137`, which writes `{name: callable(): T, …}` instead. The
      readers to delete are `callable_shape_var` (`crates/nvs-types/src/generics.rs:146`) and its
      caller in `check_generic_args` (`crates/nvs-types/src/expr/args.rs:1318`); the walk that
      replaces them takes each field's callable return type and assembles a shape with the same
      names, in `substitute`'s own single pass. `rule:concurrency/all-answers-a-typed-shape` and
      `rule:types/callable-signature`; `no_registry_row_names_a_callable_binding_site_variant`
      (`crates/nvs-stdlib/src/registry.rs:3349`) asserts the whole list and goes to `[]` the day
      this lands.
- [ ] **`Task::all`'s written-literal restriction is removed, in the checker and in the spec.**
      `bind_callable_shape` (`crates/nvs-types/src/expr/args.rs:1389`) is the only reader of a
      field, and `E0773`/`E0774` go with it; the spec's own sentence is
      `docs/spec/01-core-library.md:1301`. A field holding a `callable(): T` variable now carries
      what the field needs, which is
      `rule:concurrency/an-all-field-must-be-a-written-fn-literal`'s own condition for lifting —
      that rule is amended in the same slice, not left standing.

## Backlog

- An option bag's `{by?: callable}` / `{comparator?: callable}` is still bare — seven fields across
  `Core\Arr`'s four bags, `Core\Cli`'s three and `Core\Out`'s one. Whether a bag's field can carry a
  member variable through substitution is untested; `crates/nvs-stdlib/src/arr.rs:2010` is the first.
- `examples/typed-callable.nvs` is stage 6's fixture and is still missing —
  `docs/agent/loop-goal.toml:4740` freezes its four output lines.
- Stage 6's codegen half: a proven call site emits no `check_param_tags` — `docs/agent/loop-goal.md`
  stage 6.
- `cache::tests::a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names` (`nvs-cli`)
  failed once under `verify.py`'s parallel load and passed alone; it is timing-sensitive, not this
  work.
