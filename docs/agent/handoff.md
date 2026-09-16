# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4 and 5 are done, and stage 6 is open on `Core\Reflect`.**
`ClassInfo::construct` is the third acting member ADR 0019 § 2 names and it is on disk: it resolves
the described class by name through `Ctx::class_desc` and hands it to
`nvs_runtime::construct_erased_from` (`crates/nvs-runtime/src/dispatch.rs:479`), which allocates and
then routes the constructor through `call_erased_method_from`'s own check with this call site. So a
`private` constructor builds from its own class's bodies and is refused everywhere else, in the erased
door's own sentence, and `$arguments` meets the constructor's arity and tags at that one check.
`registry.rs:2991` carries its `CALL_SITE_MEMBERS` row. Nothing is blocked.

Four of stage 6's five named cases are green; the hooked write is the one left, and **its premise was
wrong in the last handoff**: a `set` hook is not reachable from a runtime descriptor at all. A hook
compiles to an ordinary function labelled `Class::$prop::set`
(`crates/nvs-types/src/signatures.rs:395`), but `ClassLayout::methods`
(`crates/nvs-types/src/layout.rs:102`) carries methods with a body and nothing else, so no hook has a
`MethodRow` and `ClassDesc` has no other field holding one. Running the hook on a reflective write is
therefore a four-crate slice, which is the group below and why this session took one item.

`every_reflect_info_class_the_record_names_is_registered` is still deliberately unwritten:
`crates/nvs-stdlib/src/reflect.rs:116` gap 1 names what each of the five absent classes waits on, and
it lands with the last of them.

## Next group

**Stage 6: the `set` hook, carried down to the descriptor** — one file set:
`crates/nvs-types/src/layout.rs`, `crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/lib.rs`,
`crates/nvs-runtime/src/object.rs` and `tests/conformance/core/`. The goal's § *Standing decisions*
pre-authorizes it — "a reflective write runs the `set` hook", priority 2 over the erased store's
simplicity — and `rule:classes/property-hooks` is the rule.

- [ ] **A class's hooks reach its runtime descriptor** — a per-class roster of
      `(property name, hook label)` beside the method one, flattened the same way:
      `crates/nvs-types/src/layout.rs:102` is where it is built, `crates/nvs-ir/src/ir.rs:174` copies
      it down, and both binders join the address on — `crates/nvs-codegen/src/lib.rs:956` (the AOT
      `bind`, which spells the label `{declaring}::{method}`, already exactly
      `nvs_types::signatures::hook_label`'s format at `crates/nvs-types/src/signatures.rs:395`) and
      `crates/nvs-codegen/src/lib.rs:2018` (`bind_method_tables`, the JIT one). Keep it off
      `ClassDesc::method_row`: `rule:classes/property-hooks` makes a hook an accessor and not a
      method, and `Core\Reflect\ClassInfo::methods` reads that roster.
- [ ] **A write through an erased receiver runs the `set` hook** — `crates/nvs-runtime/src/object.rs:3897`
      is `write_erased_property`, whose doc comment states the gap in its own words; the hook call
      replaces the slot store, and the observer step below it already reads the committed slot back,
      which is what `rule:classes/property-hooks` asks for. Both callers close at once — the plain
      erased write and `Core\Reflect\ClassInfo::set` — so `crates/nvs-stdlib/src/reflect.rs:143`
      gap 3 is struck in the same slice.
- [ ] **The named case** — `tests/conformance/core/reflect-a-hooked-property-written-reflectively-runs-its-set-hook.nvst`,
      stage 6's fourth case, asserting the hook's transform through `ClassInfo::set` and through an
      ordinary erased write beside it. `crates/nvs-stdlib/src/reflect.rs:1290` is `ClassInfo::set`'s
      body, which needs no edit if the runtime closes it.

## Backlog

- The erased **read** bypasses a `get` hook the same way — `crates/nvs-runtime/src/object.rs:3619`.
- `PropertyInfo` and the four `*Info` classes left — `crates/nvs-stdlib/src/reflect.rs:116` gap 1.
- A `protected` member reached from a subclass is refused — `crates/nvs-stdlib/src/reflect.rs:133` gap 2.
- `Core\Reflect\ClassInfo::methods` lists `constructor` like any other row; whether a constructor
  belongs in a method roster is unasked — `docs/decisions/0019.md` § 1.
