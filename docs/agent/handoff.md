# Handoff

## State

**Goal `m4-refusals` — Stage 7's shape row is landed.** `$x is {a: int}` lowers and walks the
subject's own fields; the `iterable`, `callable`, union and intersection rows were already there.
`python tools/holes.py` reports **3** refusal sites, `UNATTRIBUTED: 0`, **15** guarded, and
`crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **3** to match — unchanged, because the one panic
that carried the shape gap still stands for the two rows left under it.

- **The probe is an `InstKind` of its own, not a third `AbsentKey` arm.** That enum says what an
  absent *read* answers and every arm of it answers the field's value; this answers a `bool` and is
  infallible, so it carries no error edge — `crates/nvs-ir/src/ir.rs:909` is the variant and
  `crates/nvs-runtime/src/object.rs:3378` the helper. `Tag::Unset` could not be handed back as
  absence instead: `crates/nvs-runtime/src/value.rs:221` makes it a storage state that never becomes
  an expression's value.
- **The read and the probe cannot disagree**, both answering off `slot_state`
  (`crates/nvs-runtime/src/object.rs:3743`), which is the `present`/`absent`/`unwritten` split the
  three throws were already making by hand. The agreement is asserted over every state rather than
  case by case.
- **A shape is `TestShape::All` over one `TestShape::Field` per field, behind a `Tag(Ty::Object)`**,
  so a subject holding no object declines before a field is read. Each field's value is read at
  `Ty::Tagged`, the subject's own class not being the shape it was tested against, and the read's
  error edge on the probe-proven side is unreachable.
- **Stage 7 is not finished.** Its check also names
  `tests/conformance/lang/an-is-test-against-an-array-of-a-class-walks-every-element.nvst`, which is
  the `array<Foo>` gap and is not on disk. That row and a written `callable` signature are the two
  `test_shape` still answers `None` for.
- Nothing is blocked.

## Next group

**Stage 7: the element row — `is array<Foo>`** — one file set: `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-ir/src/lower/mod.rs` and `crates/nvs-runtime/src/helpers.rs`. `docs/agent/loop-goal.md`
§ *Stage 7* is the spec and `rule:types/type-test` is the rule.

- [ ] **Decide what the element walk takes instead of a tag word** — `array_element_tags`
      (`crates/nvs-ir/src/lower/mod.rs:3849`) answers `None` for an element type no tag decides, and
      the `u64` it packs has no room for a class label, a shape or a union. The walk behind it is
      `to_array_of` (`crates/nvs-runtime/src/helpers.rs:2115`), which `as ?array<T>` shares, so
      widening it moves both spellings at once — and `rule:types/type-test` keeps `as array<Foo>`
      refused where it is written while `is array<Foo>` has to answer.
- [ ] **The `array<Foo>` row over that decision** — `crates/nvs-ir/src/lower/expr.rs:5829` builds
      `TestShape::ArrayOf` from the tag word and `crates/nvs-ir/src/lower/expr.rs:5125` emits it;
      both take the `u64` today, and the shape row beside them is the model for a member test that
      is not one comparison.
- [ ] **The stage's case** — `an-is-test-against-an-array-of-a-class-walks-every-element.nvst`, new
      beside the shape one, in the counted-rows shape
      `tests/conformance/lang/an-is-test-against-a-shape-walks-its-fields.nvst:58` uses.

## Backlog

- A written `callable` signature has no `is` row — `crates/nvs-ir/src/lower/expr.rs:5691`'s
  `# Known gaps` states it, `rule:types/callable-signature` is what it owes.
- The shape walk's `SlotGet` carries an error edge the probe above it makes unreachable; erasing one
  needs a proof the IR does not hold — `crates/nvs-ir/src/ir.rs:909`.
