# Handoff

## State

**Goal 18, stages 1 and 2 are complete; stage 3 has started at its foundation rather than at its
row.** Optional shape fields, `tainted {…}`, guarded reads and `InstKind::SlotGet`'s absence answer
all landed in earlier sessions — `rule:types/shape-type`'s optional-field paragraph and
[ADR 0157](../decisions/0157.md) § 2 own that half.

**What landed this session:** `rule:core-api/required-optional-and-nullable`'s *first* column now
exists in the codec tables. `nvs_runtime::CodecField::required` and
`nvs_types::derive::DerivedField::required` carry it, `check_constructor_parameter` reads it off the
parameter's own default, `nvs_ir::lower::codec_fields` joins it down, and
`nvs_stdlib::json::decode_field` no longer reports an absent *optional* key as "required field
missing" — it names the unfilled default, which is json.rs's own gap 3, now diagnosable instead of
merely unimplemented. Behaviour is otherwise unchanged: an absent key still fails either way.

**Stage 3's member itself is not one slice, and the handoff item that said so was wrong about the
cost.** `Core\Arr::shapeAs<T>` returns a *shape*, and nothing in the tree can hand a native helper a
shape's descriptor: `registry::WRITTEN_CLASS_MEMBERS` passes a declared class's `ClassDesc`, and
`json::decode_fields` builds through a constructor that a `$shape{…}` class does not have. The
playbook bullet is the trap; the next two groups are the decomposition. Stages 4 and 5 have not
started, and the failing acceptance check (`examples/input-shapes.nvs`) is stage 5's fixture — an
unwritten artefact, not a regression.

## Next group

**Stage 3: the shape descriptor reaches the runtime** — one file set:
`crates/nvs-types/src/derive.rs`, `crates/nvs-types/src/expr/args.rs`,
`crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-ir/src/lower/expr.rs`.

- [ ] **An inline shape written as a type argument reads as a `DerivedCodec`** — a `pub fn` beside
      `codec_ty` at `crates/nvs-types/src/derive.rs:439` turning a `Ty::Shape` into
      `DerivedField`s: `required` from `ShapeField::required` (`crates/nvs-types/src/ty.rs:400`),
      `nullable` from the field type, `param` the field's index in the interner's *sorted* order,
      which is the order `nvs_ir::lower::shape_class_label` (`crates/nvs-ir/src/lower/mod.rs:2958`)
      keys slots on. `codec_ty`'s own doc names the inline shape as the erasure it drops to
      `CodecTy::Opaque` today; that sentence is what this rewrites.
      `rule:types/shape-type` and `rule:core-api/required-optional-and-nullable` are what it cites.
- [ ] **The shape a `shapeAs<T>` call site wrote reaches lowering** —
      `crates/nvs-types/src/expr/args.rs:1456` (`written_class_of`) is the roster lookup a *class*
      goes through, and a shape needs the twin beside it rather than a widening of it: a shape has
      no `QName` to record on `ResolvedCall` (`crates/nvs-types/src/expr_table.rs:169`). Prefer
      reading the call's own checked return type in `nvs-ir` over adding a field —
      `crates/nvs-ir/src/lower/expr.rs:3212` is where the `ClassDescConst`/`ConstBool` pair is
      emitted, and `record_shape_class` at `crates/nvs-ir/src/lower/mod.rs:1793` is what must also
      record the codec. `rule:types/arrays`'s type-argument-door list is amended to name three in
      the same commit.

## Backlog

- The registry row, card, body, `address()` arm and three `.nvst` cases for `Core\Arr::shapeAs<T>`,
  `crates/nvs-stdlib/src/arr.rs:694` — blocked on the group above (goal stage 3 § 1).
- `json::decode_fields` needs a ctor-less path for a `$shape{…}` class,
  `crates/nvs-stdlib/src/json.rs:1301`; `ClassDesc::is_shape()` is the discriminator that exists.
- json.rs gap 3 is now half closed: the `required` column exists, materializing the default does
  not — it needs the constant carried onto `CodecField` (`crates/nvs-stdlib/src/json.rs:91`).
- Stage 3 § 3's collected `ParseError` is already what `decode_fields` does; the wrapper members and
  stages 4–5 are untouched (`docs/agent/loop-goal.md`).
- The pack still does not print the *next* stage's paragraph from `docs/agent/loop-goal.md`, which
  is what choosing a next group reads — a `[context]` gap in `docs/agent/loop-goal.toml`.
