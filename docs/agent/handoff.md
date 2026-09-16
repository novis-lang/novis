# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4 and 5 are done, and stage 6 is open on `Core\Reflect`.**
Both of stage 6's checks are green: the five named `.nvst` cases, and
`every_reflect_info_class_the_record_names_is_registered`, which reads ADR 0019 § 1's `*Info` roster
out of the record and holds every name on it against `registry::CLASSES`.

`Core\Reflect\PropertyInfo` is registered and carries a property's name, its visibility bit and the
type its declaration spells. **The surface call is made and recorded**: `ClassInfo::properties` is now
the complete roster, `methods`' symmetric twin, and the scope-sensitive walk keeps its own behaviour
under `readableProperties`, which is the member on `registry::CALL_SITE_MEMBERS`. `hasProperty` lands
beside `hasMethod`. `crates/nvs-stdlib/src/reflect.rs:46` § *both rosters are complete* is the home of
why two members rather than one — spec § 13's row replaces `property_exists` and `get_object_vars`
alike, and those are two answers.

`NOT_YET_BUILT` (`crates/nvs-stdlib/src/reflect.rs:2175`) is the roster's remaining four, and it is
two-sided: registering one of them fails the gate until its line is deleted. Nothing is blocked.

## Next group

**Stage 6: `Core\Reflect\ParameterInfo`, over descriptor data no crate carries yet** — one file set:
`crates/nvs-types/src/layout.rs`, `crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/lib.rs`,
`crates/nvs-runtime/src/object.rs` and `crates/nvs-stdlib/src/reflect.rs`. The road is the one
`field_types` took: layout spells it, IR copies it down, codegen hands it to the class table, the
descriptor answers it. `rule:core-classes/reflect` is the rule and ADR 0019 § 1's roster the record;
the goal's § *Standing decisions* pre-authorizes the shape.

- [ ] **A method's parameter names reach the runtime descriptor** — `nvs_runtime::MethodRow`
      (`crates/nvs-runtime/src/object.rs:516`) carries a name, an arity and `param_tags`, and no
      spelling for any parameter. Fill it on `field_types`' own road:
      `crates/nvs-types/src/layout.rs:303` is where the layout spells what it knows,
      `crates/nvs-ir/src/ir.rs:175` is the copy-down beside `Class::field_types`, and
      `crates/nvs-codegen/src/lib.rs:1452` is the `set_field_types` call a `MethodRow` twin sits
      next to. Carry the names per method row rather than per class: an arity already lives there, and
      a second class-wide vector would have no slot order to align to.
- [ ] **`Core\Reflect\ParameterInfo` registered, and `MethodInfo::parameters` beside it** —
      `crates/nvs-stdlib/src/reflect.rs:880` is the template (`PROPERTY_INFO`: three readers over
      three slots, one card each, built in `describe`), and `crates/nvs-stdlib/src/reflect.rs:2175` is
      the `NOT_YET_BUILT` line the slice deletes. A row carries a name and its declared type where
      `param_tags` can spell one; `parameterCount` stays, because a count is not a list.
- [ ] **Three `.nvst` cases per new member** — `tests/conformance/core/`, on the shape
      `tests/conformance/core/reflect-property-info-lists-a-classs-properties-with-their-visibility.nvst:19`
      walks a roster with. The trap the playbook's *Writing a test case* names applies: a case must
      spell `Core\Reflect\ParameterInfo` for its members to be counted at all.

## Backlog

- An erased **read** still goes past a `get` hook — `crates/nvs-runtime/src/object.rs:3735` owns the
  gap; closing it needs a `Ctx` threaded through `read_erased_property_hinted`, which today has none.
- `ConstantInfo`, `AttributeInfo` and `EnumInfo` are the far three: a descriptor carries no constants,
  attributes or enum cases at all — `crates/nvs-stdlib/src/reflect.rs:116` gap 1.
- A `protected` member is reached reflectively from the declaring class alone, not from a subclass —
  `crates/nvs-stdlib/src/reflect.rs` gap 2, which fails closed.
- The exception tree's slots carry no declared type *text* — `nvs_types::error_lib` types them as
  interned ids, so `ClassDesc::field_type` answers `None` for every `Throwable` slot and a
  `PropertyInfo` over a caught error shows it as `null`.
- `docs/decisions/0126.md:75` names `ClassInfo::properties` for a *visibility-filtered* set, which is
  `readableProperties` now. The record is frozen rationale; the shape-key slice takes the member from
  `rule:security/reflection-enforces-visibility` rather than from that sentence.
