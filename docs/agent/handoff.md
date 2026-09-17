# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `crates/nvs-stdlib/src/reflect.rs`'s gap 2 is
built and its item deleted: `python tools/owners.py --closes decided-closures` names 16 where it
named 17. A `protected` member is now reached reflectively from exactly the sites an ordinary
access reaches it from, which is the whole of `rule:security/reflection-enforces-visibility`
rather than the closed-fails-safe half.

**The visibility *level* travels now, where one readable/not bit did.** `nvs_types::layout` reads
`protected` off the declaration beside `public`; the pair rides `nvs_ir::ir::Class` and
`nvs-codegen` to `ClassDesc::field_is_protected` and `MethodRow::protected`. The one answer both
doors ask is `Ctx::field_is_visible_from` / `Ctx::method_is_visible_from` in the new
`crates/nvs-runtime/src/ctx/visibility.rs`, which replaces the site-equality each of the five call
sites had written for itself — `ClassInfo::get`, `::set`, `::readableProperties` and, through
`call_erased_method_from`, `::call` and `::construct`.

**The walk is exact in both directions, which is what the second bit bought.** A site that *is* the
subject's class reaches its `protected` members because it inherits them; an ancestor reaches one
when it declares it, which is what the module's `declares` argument asks; an unrelated class that
spells the same property name is refused, so the relation is asked and never the spelling alone.
`private` still wants equality, and a site inside no class is still outside everything.

**What is left in this file is gap 1**, and it is the bigger half: `ConstantInfo` and
`AttributeInfo` each need descriptor data no `ClassDesc` carries at all.

## Next group

**Stage 4: `crates/nvs-stdlib/src/reflect.rs`'s last gap, one join at a time** — one file set, and
it is the channel this session just widened by one bit: `crates/nvs-types/src/layout.rs`,
`crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/lib.rs`, `crates/nvs-runtime/src/object.rs`,
`crates/nvs-stdlib/src/reflect.rs` and the cases under `tests/conformance/core/`.

- [ ] **`crates/nvs-stdlib/src/reflect.rs:152` — gap 1a: `Core\Reflect\ConstantInfo`.** A
      descriptor carries no class constants, so the member is a join before it is a class, and the
      join is this session's shape exactly: a roster beside `crates/nvs-types/src/layout.rs:95` in
      `ClassLayout`, beside `crates/nvs-ir/src/ir.rs:205` in `ir::Class`, a setter pair at
      `crates/nvs-codegen/src/lib.rs:1542` and a list on `crates/nvs-runtime/src/object.rs:476`'s
      descriptor. **Decide the carried form in the slice and state what it spends**
      (`rule:programs/memory-priority`): a constant is a *value*, not a bit, so it is either the
      declaration's spelling or something the runtime can hand back as a `Value`. Then the five
      edits of `docs/agent/conventions.md` § *A `Core` member* and three `.nvst` cases.
- [ ] **`crates/nvs-stdlib/src/reflect.rs:152` — gap 1b: `Core\Reflect\AttributeInfo`**, through
      the same join — `crates/nvs-types/src/layout.rs:95`, `crates/nvs-ir/src/ir.rs:205`,
      `crates/nvs-codegen/src/lib.rs:1542`, `crates/nvs-runtime/src/object.rs:476` — and the same
      five edits. An attribute
      is a name plus its arguments, so locate where `nvs_types` holds the attribute table first —
      `rule:core-classes/derive-generates-what-is-missing` already records a per-class field list on
      the descriptor, and whether that pass is the place to hang this is the first question.

## Backlog

- An ordinary erased call — `mixed $x = $obj; $x->m()` — still passes `site: None`, so a
  `protected` method is refused through it even from inside the hierarchy;
  `crates/nvs-runtime/src/dispatch.rs`'s `call_erased_method` doc owns that as a decision, and the
  level reaching the descriptor is what would now make it answerable.
- `docs/agent/carried-gaps.md` is the home for anything that must outlive this goal.
