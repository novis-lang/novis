# Handoff

## State

**Item 13's first half is landed: an abandoned generator has an entry point to be
unwound through, and nothing calls it yet.** `{name}$gen::unwind` is on every
generator's state class and in its method table, `gen#unwind` is the flag it raises,
and each suspension point inside a `finally`-owning region branches on that flag into
a copy of exactly what `return;` lowers to there. The mechanism's one home is
`mwl_ir::lower::generator`'s `lower_generator` doc, § *An abandoned generator runs its
`finally`*; ADR 0028 § 2 carries the paragraph saying it is not a destructor.

- **What is missing is the release path, and it is one call plus a resurrection.**
  `mwl_runtime::object::dismantle` (`crates/mwl-runtime/src/object.rs:1314`) sweeps
  every field slot and then deallocates; the `unwind` call belongs **before** that
  sweep, since the `finally` body reads the parked locals out of those very slots.
  `unwind` **borrows** argument 0 for this — its own doc comment argues why — so
  `dismantle` never has to hand over a reference it does not have and the count never
  crosses zero twice.
- **`unwind` is looked up by name in the class's method table**, deliberately: the
  flag's slot index and the entry switch's encoding are facts of the transform, and a
  runtime that knew either would be holding a copy of it.
- **A generator with no `finally` lowers exactly as it did.** The branch is emitted
  only where `try_stack` holds a frame that owns one, so the only unconditional cost
  is the one extra field slot — eight bytes per suspended generator, O(in-flight).
- **`orient.py`'s pack was complete for this slice**, minus one thing worth a
  selector: nothing in it named `crates/mwl-runtime/src/object.rs`, which the group's
  own second slice anchors into. `[context] modules` could gain a `mwl-runtime` entry.

## Next group

**Item 13's remaining two thirds, in order.** The file set:
`crates/mwl-runtime/src/object.rs`, then `tests/`. Slice 1 is a runtime slice and is
the delicate one; slices 2 and 3 only become writable once it lands.

- [ ] **The release path calls the entry point.** In `dismantle`
      (`crates/mwl-runtime/src/object.rs:1314`), before the `0..field_count` sweep at
      `:1324`: ask the descriptor whether its class answers `"unwind"`, and call it
      with the object pointer as argument 0. The count is already zero here, which is
      the whole reason `unwind` borrows — see its doc comment in
      `crates/mwl-ir/src/lower/generator.rs:@lower_generator_unwind`. Two things to
      settle in that session, both cheaply: whether the method-table lookup is a name
      probe or a `ClassDesc` field set at `mwl_stdlib::instance`-time like
      `ClassDesc::renderer` already is (that precedent is in the plan's *Open now*),
      and where a throw escaping the `finally` goes when the caller is a release
      rather than a call site. Prefer the safe option and record it.
- [ ] **The two named cases**, both listed by `python tools/holes.py --cases`, once
      the path above runs. `.agent-tmp/gen-unwind.mwl` is a scratch generator with a
      `finally` around two `yield`s and one after the region; adding a `break` to its
      `foreach` is the abandoned shape. The differential twin belongs in
      `tests/differential/iter/`, beside the two `…-matches-php` cases `mwl-ir` gap 18
      already names.
- [ ] **Fallback, pre-authorized** and now clearly not needed: the state machine
      expresses this fine, so the "keep the divergence and pin it" branch in
      `loop-goal.md` § *Standing decisions* is dead for item 13.

## Backlog

- `Iterator<T>::current` before the first `advance()` or after a `false` still reads a
  `null` slot as a `T` — `lower_generator_current`'s own doc owns it, ADR 0053 § 1
  says it throws.
- A `yield` inside a `finally` body now lowers twice, once per copy, and the unwind
  copy's resume arm is reachable from the entry switch. PHP fatals on it; nothing
  refuses it here. `crates/mwl-ir/src/lower/generator.rs:@lower_yield`.
- Stage 3's remaining items and the ten Stage 5–7 named cases `python tools/holes.py
  --cases` lists are the frontier behind item 13.
- `holes.py` still attributes `emit.rs`'s nine generic catch-alls to Stage 0 item 1 by
  *file* rather than by shape — a tool question, not a language one.
