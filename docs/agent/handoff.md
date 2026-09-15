# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4 and 5 are done, and stage 6 is open on `Core\Reflect`.**
An acting member now faces the class its call site is inside: `ClassInfo::properties`, `get`, `set`
and `call` take the enclosing class as a trailing constant of the call, which
`crates/nvs-stdlib/src/registry.rs:2986`'s `CALL_SITE_MEMBERS` owns the ABI of and no program can
write. The walk answers two ways off one description — every declared property from inside the
described class, the public ones from anywhere else — and `call` hands the site to
`nvs_runtime::call_erased_method_from`, so a `private` method runs from its own class's bodies and
is refused everywhere else with the sentence the ordinary erased call gives. Nothing is blocked.

Two of stage 6's five named cases are written and green. The three left are the constructor, the
hooked write and the method roster's own case, which exists already — so the check turns green when
the two acts below land. `every_reflect_info_class_the_record_names_is_registered` is still
deliberately unwritten: `crates/nvs-stdlib/src/reflect.rs:116` gap 1 names what each of the five
absent classes waits on, and it lands with the last of them.

A `protected` member reached reflectively from a subclass is refused where an ordinary call there is
allowed — `crates/nvs-stdlib/src/reflect.rs:133` gap 2 is the home, and closing it is a second
visibility bit carried down from `nvs_types::layout`. It fails closed, so it is a gap and not a hole.

## Next group

**Stage 6: the two acts left, then the roster** — one file set: `crates/nvs-stdlib/src/reflect.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-runtime/src/` and `tests/conformance/core/`. Both
acts are pre-authorized by the goal's § *Standing decisions*, and `rule:security/reflection-enforces-visibility`
is the rule for the first two.

- [ ] **A constructor invoked reflectively runs its visibility check** — the third acting member
      ADR 0019 § 2 names, and `crates/nvs-stdlib/src/reflect.rs:143` is gap 3, which says it is
      absent. `crates/nvs-runtime/src/dispatch.rs:851` is `construct_and_call`, the path an ordinary
      `new` through an erased class reaches, so the member is the five edits plus a row on
      `CALL_SITE_MEMBERS` — the visibility question is the constructor's own `public` bit, asked
      against the site that constant carries. `tests/conformance/core/reflect-a-constructor-invoked-reflectively-runs-its-visibility-check.nvst`
      is stage 6's third named case; its spelling comes from `rule:core-api/verb-lexicon`.
- [ ] **A reflective write runs the `set` hook** — the goal's standing decision fixes it: writing
      past a hook is a correctness hole, and priority 2 outranks the simplicity of the erased store.
      `crates/nvs-runtime/src/object.rs:3897` is `write_erased_property`, which reaches storage and
      not the hook call a known class's write lowers to; what it is missing is the hook's address on
      `nvs_runtime::ClassDesc`, the way `renderer()` carries one. `rule:classes/property-hooks` is
      the rule, and `tests/conformance/core/reflect-a-hooked-property-written-reflectively-runs-its-set-hook.nvst`
      is the fourth named case.
- [ ] **`PropertyInfo`, then the four `*Info` classes left** — `crates/nvs-stdlib/src/reflect.rs:116`
      gap 1 names what each waits on, and `PropertyInfo` is the near one: its row carries the
      visibility bit the two property slots now hold separately, so landing it collapses
      `ALL_PROPERTIES_SLOT` back into one roster. The last of the five is what
      `every_reflect_info_class_the_record_names_is_registered` lands with.

## Backlog

- `crates/nvs-stdlib/src/json.rs` gap 1 is down to an `array<T>` of inline shapes (stage 5's tail).
- The record producers, CSV streaming, CLDR and `Core\Metrics` are stages 7 onward of this goal.
- `nvs-server`'s `a_fleet_lease_is_renewed_while_its_run_is_in_flight` fails under a full-tree `test`
  and passes alone — a shared-resource flake, not this goal's.
