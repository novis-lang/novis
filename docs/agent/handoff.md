# Handoff

## State

**Goal 18, stages 1 and 2 are complete; stage 3 has landed its first half.** Optional shape fields,
`tainted {…}`, guarded reads, `InstKind::SlotGet`'s absence answer and the codec tables' `required`
column all stand — `rule:types/shape-type`'s optional-field paragraph, `rule:core-api/required-optional-and-nullable`
and [ADR 0157](../decisions/0157.md) § 2 own that half.

**What landed this session:** `nvs_types::derive::shape_codec` — an inline shape read as a
`DerivedCodec` without a declaration to read it off. `required` comes from the written `?`,
`nullable` from the field type's own `null` arm, and `param` from the interner's *sorted* field
order, which is the order `nvs_ir::lower::shape_class_label` keys a shape class's slots on.
`codec_ty` and `enum_cases` now take `(&TypeInterner, &EnumTable)` rather than the crate-private
`Env`, which is what lets the new door be `pub`.

**Stage 3's remaining half has a fork in it that the old handoff item did not see.** A shape class is
keyed on field *names* alone, so `{n: int}` and `{n: string}` are one class and one `ClassDesc`, and
the descriptor a `WRITTEN_CLASS_MEMBERS` call site passes in slot 0 therefore cannot carry a shape's
per-field wire types. The playbook bullet is the trap; the group below is the fork. Stages 4 and 5
have not started, and the failing acceptance check (`examples/input-shapes.nvs`) is stage 5's
fixture — an unwritten artefact, not a regression.

**The pack's `[context.stage.3] modules` is missing `nvs-ir/src/lower/call.rs`**, which is where
`WRITTEN_CLASS_MEMBERS` lowering lives and where the group below spends its first read.

## Next group

**Stage 3: a shape's wire types reach the helper** — one file set:
`crates/nvs-ir/src/lower/call.rs`, `crates/nvs-ir/src/lower/mod.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-runtime/src/object.rs`.

- [ ] **Pick the carrier for a shape's `CodecField` list, and write it down where the reader looks
      first** — the `WRITTEN_CLASS_MEMBERS` path puts a `ClassDescConst` in slot 0
      (`crates/nvs-ir/src/lower/call.rs:713`, `crates/nvs-stdlib/src/json.rs:141`), and a shape's
      descriptor is shared across every shape with those field names
      (`crates/nvs-ir/src/lower/mod.rs:2957`). Either the list rides beside the descriptor as its
      own call-site constant, or a shape reaching this door mints a class keyed on names *and*
      types — and the second breaks the one-class-per-field-set guarantee
      `crates/nvs-stdlib/src/json.rs:583` and `crates/nvs-stdlib/src/task.rs:52` both rest on, so
      prefer the first. `rule:types/shape-type` and `rule:core-api/required-optional-and-nullable`
      are what it cites; the reasoning belongs in `nvs-ir`'s own module doc unless it turns into a
      rule change, in which case ADR 0159 is the one number this goal may open.
- [ ] **Emit that carrier at a call site that wrote an inline shape as its type argument** —
      `nvs_types::derive::shape_codec` (`crates/nvs-types/src/derive.rs:423`) is the producer and is
      tested; what is missing is the recording, beside where a declared class's codec is recorded
      (`crates/nvs-types/src/expr_table.rs:1181`), keyed so two shapes with one label do not
      collide, and the read back out at `crates/nvs-ir/src/lower/mod.rs:748`.
      `rule:core-classes/derive-field-list` is what it cites.

## Backlog

- `Core\Arr::shapeAs<T>`'s registry row and its three conformance cases — stage 3's member itself,
  once the carrier above exists; `docs/agent/loop-goal.md` § stage 3.
- `codec_ty` still erases a nested inline shape to `CodecTy::Opaque`; `crates/nvs-types/src/derive.rs:423`'s
  own doc says so, and `nvs_stdlib::json`'s gap owns the decoder.
- Stage 4's diagnostics and stage 5's `examples/input-shapes.nvs` fixture — `docs/agent/loop-goal.md`.
