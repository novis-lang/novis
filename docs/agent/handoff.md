# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4 and 5 are done.** A `decimal`, a `Core\Time\Instant` and
now an inline shape reached as a *field* all cross the JSON wire, and all three of stage 5's checks
are green. A shape field erases to `nvs_runtime::CodecTy::Shape`, which carries the two resolved
pointers such a decode needs — the class a literal of those field names builds, on `CodecField::class`,
and the contract holding the field types that label cannot, on the new `CodecField::shape`.
`crates/nvs-stdlib/src/json.rs`'s gap 1 now covers only an `array<T>` of shapes. Nothing is blocked.

The end-to-end path is exercised by
`tests/conformance/core/json-decode-as-fills-an-inline-shape-field.nvst`, which is the only check on
the `nvs-ir`/`nvs-codegen` half — the class a shape field names is synthesized from the derived codec
rather than from a literal, and the contract is published beside it.

## Next group

**Stage 6: `Core\Reflect`** — one file set: `crates/nvs-stdlib/src/reflect.rs`,
`crates/nvs-stdlib/src/registry.rs` and `tests/conformance/core/`. Every case stage 6 names is still
absent, so each item below lands its own. `rule:tooling/reflection-and-source-parsing-are-core-features`
is the rule, and the goal's § *Standing decisions* has already settled the call-site question and the
hook one.

- [ ] **The `*Info` roster § 1 names** — `crates/nvs-stdlib/src/reflect.rs:73` is gap 1: a description
      names its properties and no methods, so `get_class_methods`/`method_exists` have no answer.
      `docs/spec/01-core-library.md` § 20 is the roster's one home, and each class is a row in
      `crates/nvs-stdlib/src/registry.rs`. The check wants
      `every_reflect_info_class_the_record_names_is_registered` in `nvs-stdlib`, which does not exist,
      and `tests/conformance/core/reflect-method-info-lists-a-classs-methods-with-their-visibility.nvst`.
      Adding a row-less `Core` symbol owes four edits — the playbook's own bullet over
      `crates/nvs-stdlib/src/lib.rs:every_registered_member_has_an_implementation_address`.
- [ ] **The acting members face the call site's own class** — `crates/nvs-stdlib/src/reflect.rs:78` is
      gap 2: the walk answers the same from inside the described class as from outside, because a
      native member has no view of its caller. The standing decision supplies the enclosing class as a
      hidden argument no program can write, and falls back to "from outside" if it cannot be threaded.
      Cases: `reflect-a-private-method-called-from-outside-fails-like-an-ordinary-call.nvst` and
      `reflect-the-walk-from-inside-a-class-sees-what-an-ordinary-read-there-sees.nvst`.
- [ ] **A reflective write runs the `set` hook, and a constructor can be invoked** —
      `crates/nvs-stdlib/src/reflect.rs:86` is gap 3's second half: the write reaches storage through
      `nvs_runtime::write_erased_property`, which is the erased store and not the hook call
      (`rule:classes/property-hooks`). Cases:
      `reflect-a-constructor-invoked-reflectively-runs-its-visibility-check.nvst` and
      `reflect-a-hooked-property-written-reflectively-runs-its-set-hook.nvst`.

## Backlog

- An `array<T>` of inline shapes still erases to `Opaque` — `crates/nvs-stdlib/src/json.rs` gap 1.
- A `?T` nested inside an inline shape is refused at the declaration — `rule:core-classes/derive-field-list`
  admits it, `crates/nvs-types/src/derive.rs`'s `json_reachable` does not.
- `crates/nvs-runtime/src/budget.rs:87` gap 1 is orphaned, and no end-to-end `nvs serve` runaway test
  exists — `docs/plan/m7.md`.
