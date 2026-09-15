# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4 and 5 are done, and stage 6 is open on `Core\Reflect`.**
The first of § 1's remaining `*Info` classes is landed: `Core\Reflect\MethodInfo` is registered, and
a description reached through either door carries a complete method roster — one row per declared or
inherited method, each with its name, its `public` bit and its declared parameter count.
`Core\Reflect\ClassInfo::methods` and `::hasMethod` are what read it, replacing `get_class_methods`
and `method_exists`. Nothing is blocked.

The roster is **complete rather than visibility-filtered**, and `crates/nvs-stdlib/src/reflect.rs`'s
§ *the method roster is complete* is that decision's home: ADR 0019 § 2 divides reading metadata,
which is always available, from acting on a member, which faces the check — so a `private` method is
named with its bit and `ClassInfo::call` is still the only door that acts. That is also what makes a
refusal distinguishable from a misspelling, which `rule:core-classes/reflect` asks for.

`every_reflect_info_class_the_record_names_is_registered` is **not written**, on purpose: five of
ADR 0019 § 1's classes are still absent, and a test asserting the roster with a shrinking exemption
list would turn stage 6's second check green while the roster was short. It lands with the last of
the five. `crates/nvs-stdlib/src/reflect.rs:94` gap 1 names what each of the five is waiting on.

## Next group

**Stage 6: the acting members face the call site** — one file set: `crates/nvs-stdlib/src/reflect.rs`,
`crates/nvs-types/src/`, `crates/nvs-ir/src/lower/` and `tests/conformance/core/`. The goal's
§ *Standing decisions* has already settled both calls: the enclosing class arrives as a hidden
argument no program can write, and the safe fallback is to treat every reflective act as coming from
outside. `rule:security/reflection-enforces-visibility` is the rule.

- [ ] **The walk and the acting members face the call site's own class** —
      `crates/nvs-stdlib/src/reflect.rs:111` is gap 2: a native member has no view of its caller, so
      `properties`, `get`, `set` and `call` all answer the outside-the-class question. Thread the
      enclosing class in as the hidden argument the standing decision names, then
      `tests/conformance/core/reflect-the-walk-from-inside-a-class-sees-what-an-ordinary-read-there-sees.nvst`
      is stage 6's second named case. `crates/nvs-stdlib/src/reflect.rs:757` is `describe`, which is
      where the property walk's filter is applied.
- [ ] **A reflective write runs the `set` hook, and a constructor can be invoked** —
      `crates/nvs-stdlib/src/reflect.rs:116` is gap 3. The write reaches storage through
      `nvs_runtime::write_erased_property`, which is the erased store rather than the hook call a
      known class's write lowers to; the standing decision settles that priority 2 outranks the
      erased store's simplicity. Cases:
      `tests/conformance/core/reflect-a-hooked-property-written-reflectively-runs-its-set-hook.nvst`
      and `tests/conformance/core/reflect-a-constructor-invoked-reflectively-runs-its-visibility-check.nvst`.
- [ ] **`PropertyInfo`, then the four `*Info` classes left** —
      `crates/nvs-stdlib/src/reflect.rs:94` gap 1 names what each waits on.
      `crates/nvs-stdlib/src/reflect.rs:659` (`METHOD_INFO`) is the shape to copy, and
      `crates/nvs-runtime/src/object.rs:1044` (`method_at`) is the descriptor accessor its twin will
      need. `every_reflect_info_class_the_record_names_is_registered` lands with the last of them,
      and stage 6's first named case —
      `tests/conformance/core/reflect-a-private-method-called-from-outside-fails-like-an-ordinary-call.nvst`
      — is still unwritten and does not depend on any of this.

## Backlog

- `ClassInfo::properties` names only what the calling site may read, where `methods` names
  everything — `crates/nvs-stdlib/src/reflect.rs:94` gap 1 says `PropertyInfo` re-asks it.
- `crates/nvs-stdlib/src/json.rs` gap 1 is down to an `array<T>` of inline shapes (stage 5's tail).
- The record producers, CSV streaming, CLDR and `Core\Metrics` are stages 7 onward of this goal.
