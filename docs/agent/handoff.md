# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4 and 5 are done, and stage 6 is open on `Core\Reflect`.**
Stage 6's five named `.nvst` cases are all on disk and green: a hooked property written reflectively
now runs its `set` hook, because a class's hooks reach its runtime descriptor.
`nvs_types::layout::ClassLayout::hooks` collects `(property, hook label, is the `set` accessor)` and
flattens it on `methods`' precedence, `nvs_ir::ir::Class::hooks` copies it down, both binders join the
compiled address on, and `nvs_runtime::ClassDesc::hook_row` answers it — which
`write_erased_property` calls before it ever reaches a slot. Nothing is blocked.

**A floor regression was the acceptance failure, not unwritten work.** Commit `abdc729b6` had renamed
the test a carried floor check pins; the name is restored and this goal's own stage-5 check names it.
The playbook bullet above is the general shape.

What stage 6 still owes is `every_reflect_info_class_the_record_names_is_registered`:
`crates/nvs-stdlib/src/reflect.rs:116` gap 1 names the five absent `*Info` classes and what each waits
on. The near pair is the group below, and it is the journey this session just built, one datum over.

## Next group

**Stage 6: `PropertyInfo` and `ParameterInfo`, carried down the same four crates** — one file set:
`crates/nvs-types/src/layout.rs`, `crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/lib.rs`,
`crates/nvs-runtime/src/object.rs` and `crates/nvs-stdlib/src/reflect.rs`. `rule:core-classes/reflect`
is the rule; the goal's § *Standing decisions* pre-authorizes the shape, and
`crates/nvs-stdlib/src/reflect.rs:116` gap 1 says each of these waits on descriptor data rather than
on a decision.

- [ ] **A property's declared type reaches its descriptor as a printable name** — the roster the hook
      one is now beside: `crates/nvs-types/src/layout.rs:61` builds it,
      `crates/nvs-ir/src/ir.rs:186` copies it down, `crates/nvs-codegen/src/lib.rs:1213` carries it to
      `nvs_runtime::ClassTable`, and `crates/nvs-runtime/src/object.rs:290` holds it. A
      `nvs_runtime::Tag` is not enough — `?int`, `array<string>` and a class are one tag or none.
- [ ] **`Core\Reflect\PropertyInfo` is registered over it** — the five edits, roster at
      `crates/nvs-stdlib/src/reflect.rs:373`, beside `METHOD_INFO` at
      `crates/nvs-stdlib/src/reflect.rs:754`. Its walk answers what the calling site may read, which is
      `nvs_core_reflect_class_info_properties`' rule at `crates/nvs-stdlib/src/reflect.rs:392`.
- [ ] **`Core\Reflect\ParameterInfo` beside it** — a parameter's *name* has no home below the front
      end either, so it travels the first item's road; `crates/nvs-stdlib/src/reflect.rs:116` gap 1 is
      what it closes half of.

## Backlog

- An erased **read** still goes past a `get` hook — `crates/nvs-runtime/src/object.rs:3735` owns the
  gap; closing it needs a `Ctx` threaded through `read_erased_property_hinted`, which today has none.
- `ConstantInfo`, `AttributeInfo` and `EnumInfo` are the far three: a descriptor carries no constants,
  attributes or enum cases at all — `crates/nvs-stdlib/src/reflect.rs:116` gap 1.
- A `protected` member is reached reflectively from the declaring class alone, not from a subclass —
  `crates/nvs-stdlib/src/reflect.rs` gap 2, which fails closed.
- `[context] modules` names none of `crates/nvs-runtime/src/object.rs`,
  `crates/nvs-codegen/src/lib.rs` or `crates/nvs-ir/src/ir.rs`, which stage 6's work has now spent
  three sessions in; the ledger has flagged it each time and it is left for a person to narrow.
