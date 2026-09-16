# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4 and 5 are done, and stage 6 is open on `Core\Reflect`.**
Stage 6's five named `.nvst` cases are all on disk and green. A property's **declared type** now reaches
the runtime descriptor as a printable name, on the hook roster's own road:
`nvs_types::layout::ClassLayout::field_types` spells it from the declaration that wrote it, aligned to
`fields`' slot order; `nvs_ir::ir::Class::field_types` copies it down, `nvs-codegen` hands it to
`nvs_runtime::ClassTable::set_field_types` under the length guard its visibility twin uses, and
`nvs_runtime::ClassDesc::field_type` answers it — `None` for a slot no declaration laid out.

`crates/nvs-stdlib/src/reflect.rs`'s gap 3 is struck: last session's hook work made a reflective write
run the `set` hook, and the gap still said it did not. Gap 1 now names the class registration itself as
`PropertyInfo`'s one remaining blocker. Nothing is blocked.

**The next item is not the five mechanical edits the last handoff called it.** `ClassInfo::methods`
answers `array<MethodInfo>` while `ClassInfo::properties` answers `array<string>` filtered to the
calling site, so registering `PropertyInfo` first decides which of those two shapes the property half
takes. The item below names the evidence on both sides.

## Next group

**Stage 6: the `*Info` classes, over descriptor data that now exists** — one file set:
`crates/nvs-stdlib/src/reflect.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/instance.rs` and `tests/conformance/core/`. `rule:core-classes/reflect` is the
rule; ADR 0019 § 1's roster and § 2's *reading metadata is always available* are the record, and the
goal's § *Standing decisions* pre-authorizes the shape.

- [ ] **`Core\Reflect\PropertyInfo` registered, and the surface call made before the five edits** —
      `crates/nvs-stdlib/src/reflect.rs:749` is the template (`METHOD_INFO`: three readers over three
      slots, one card each), `crates/nvs-stdlib/src/reflect.rs:855` is the walk that would fill it and
      already builds both the visible and the declared name lists,
      `crates/nvs-stdlib/src/registry.rs:2017` is the one-line registration and `:2987` the member
      roster. The call to make first: either `properties` becomes `methods`' symmetric roster — ADR
      0019 § 2 makes metadata readable whatever the visibility, and
      `tests/conformance/core/reflect-describes-a-class-by-the-properties-visible-from-outside.nvst`
      is a `rule:core-classes/reflect` guard case that is then amended *with* the rule in the same
      slice — or a second member lands beside it and the two rosters keep answering differently. Read
      the module doc's § *the method roster is complete* (`crates/nvs-stdlib/src/reflect.rs:46`)
      first; it owns why they differ today. The type each row carries is
      `nvs_runtime::ClassDesc::field_type` (`crates/nvs-runtime/src/object.rs:1032`).
- [ ] **`Core\Reflect\ParameterInfo` beside it** — a parameter's *name* is in no descriptor:
      `nvs_runtime::MethodRow` (`crates/nvs-runtime/src/object.rs:516`) carries the address, the arity
      and one tag per slot and no spelling, while `nvs_types::signatures::MethodSig`
      (`crates/nvs-types/src/signatures.rs:57`) holds the names. It travels the road this session
      built, one datum over: `layout`/`signatures` → `nvs_ir::ir::Class` → `ClassTable` → `ClassDesc`.
- [ ] **`every_reflect_info_class_the_record_names_is_registered`** — the stage-6 check at
      `docs/agent/loop-goal.toml:10917`, enumerating ADR 0019 § 1's roster against
      `crates/nvs-stdlib/src/registry.rs:2012`'s `CLASSES`. Write it with the **last** Info class of
      the stage: it names seven classes and stays red until `ConstantInfo`, `AttributeInfo` and
      `EnumInfo` land too, so writing it earlier only leaves a red test in the tree.

## Backlog

- An erased **read** still goes past a `get` hook — `crates/nvs-runtime/src/object.rs:3735` owns the
  gap; closing it needs a `Ctx` threaded through `read_erased_property_hinted`, which today has none.
- `ConstantInfo`, `AttributeInfo` and `EnumInfo` are the far three: a descriptor carries no constants,
  attributes or enum cases at all — `crates/nvs-stdlib/src/reflect.rs:116` gap 1.
- A `protected` member is reached reflectively from the declaring class alone, not from a subclass —
  `crates/nvs-stdlib/src/reflect.rs` gap 2, which fails closed.
- The exception tree's slots carry no declared type *text* — `nvs_types::error_lib` types them as
  interned ids, so `ClassDesc::field_type` answers `None` for every `Throwable` slot and a
  `PropertyInfo` over a caught error will show it.
- `[context] modules` named none of `crates/nvs-runtime/src/object.rs`, `crates/nvs-codegen/src/lib.rs`
  or `crates/nvs-ir/src/ir.rs`; this session's commit touches all three, so the driver's own sweep of
  the field should close it without a person.
