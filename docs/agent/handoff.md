# Handoff

## State

**Goal 18, stages 1 and 2 are complete; stage 3 has landed the fork and the carrier's runtime end.**
Optional shape fields, `tainted {…}`, guarded reads, `InstKind::SlotGet`'s absence answer, the codec
tables' `required` column and `nvs_types::derive::shape_codec` all stand —
`rule:types/shape-type`'s optional-field paragraph, `rule:core-api/required-optional-and-nullable`
and [ADR 0157](../decisions/0157.md) § 2 own that half.

**The fork is decided and no rule changed, so ADR 0159 is still unopened.** A shape's wire contract
is a **constant of the call site**, never of its class: `shape_class_label` keys a shape class on
sorted field *names*, so `{n: int}` and `{n: string}` are one `ClassDesc`, and a shape class is
synthesized in `nvs-ir` rather than laid out in `nvs_types::layout`, so `lower_file`'s codec join has
no slot order to run one against either. The alternative — a class keyed on names *and* types — was
refused because `nvs_stdlib::json`'s shape encoder and `nvs_stdlib::task`'s `all` both rest on one
field set being one class. `crates/nvs-ir/src/lib.rs` § *Design choices* is that decision's one home,
including what it spends and the consequence that a call site naming a shape must register the
shape's class itself.

**What landed beside it:** `nvs_runtime::ShapeCodec`, owned by the unit's `ClassTable` (boxed for
`ClassTable::desc`'s address-stability reason) and reached from a call's argument slot through
`Value::shape_codec`/`as_shape_codec` — the `Tag::Null`-over-an-address convention a descriptor
already rides, with its own named pair so neither can be read as the other. Nothing produces one
yet; the next group is the emitting half.

The failing acceptance check (`examples/input-shapes.nvs`) is stage 5's fixture — an unwritten
artefact, not a regression.

## Next group

**Stage 3: the carrier is emitted and then consumed** — one file set:
`crates/nvs-types/src/expr/args.rs`, `crates/nvs-types/src/expr_table.rs`,
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-codegen/src/lib.rs`,
`crates/nvs-stdlib/src/arr.rs`, `crates/nvs-stdlib/src/registry.rs`. **In this order** — the
coverage gate fails the moment a registry row has no conformance case, so the member is last.

- [ ] **Record an inline shape's codec at the call site that wrote it** — `written_class_of`
      (`crates/nvs-types/src/expr/args.rs:1456`) answers a shape's class label and list flag instead
      of `E_TYPE_ARG_NOT_A_CLASS`, and records `nvs_types::derive::shape_codec`'s list
      (`crates/nvs-types/src/derive.rs:442`) beside `record_codec`
      (`crates/nvs-types/src/expr_table.rs:1181`) keyed by the **type argument's span**: `$shape{n}`
      is one label for `{n: int}` and `{n: string}`, which is exactly why the label cannot key it.
      The label needs a field of its own on `ResolvedCall` (`crates/nvs-types/src/expr/calls.rs:198`)
      — `written_class` is a `QName` and `$shape{…}` is not one. Inert until the roster names a
      member, so it lands green alone. `rule:types/shape-type`,
      `rule:core-api/required-optional-and-nullable`.
- [ ] **Emit the carrier, and register the shape's class with it** — a third constant beside the
      descriptor and the list flag at `crates/nvs-ir/src/lower/expr.rs:3212` and `:3441` (and
      `crates/nvs-ir/src/lower/closure.rs:621`), plus a `record_shape_class`
      (`crates/nvs-ir/src/lower/mod.rs:1793`) call so the descriptor constant resolves in a unit that
      spells that shape nowhere else. `crates/nvs-codegen/src/lib.rs:1111` materializes it through
      `ClassTable::define_shape_codec` (`crates/nvs-runtime/src/object.rs:1335`) in the same second
      pass, and `crates/nvs-codegen/src/emit.rs:597` bakes the address beside `ClassDescConst`'s.
      `crates/nvs-ir/src/lib.rs` § *A shape's wire contract* is the design it implements.
- [ ] **`Core\Arr::shapeAs<T>` — the row, the roster entry and three cases in one slice** —
      `crates/nvs-stdlib/src/arr.rs` and the five edits
      ([conventions.md](conventions.md) § *A `Core` member*), plus the roster row at
      `crates/nvs-stdlib/src/registry.rs:2382`; the body reads its contract through
      `Value::as_shape_codec` (`crates/nvs-runtime/src/value.rs:588`) and hydrates with `as`'s own
      table. `docs/agent/loop-goal.md` § *Stage 3*.

## Backlog

- `codec_ty` erases a nested inline shape to `CodecTy::Opaque` — `crates/nvs-types/src/derive.rs:442`.
- `Json::decodeAs<{…}>` should reach the same walk rather than keeping its own — goal stage 3 § 2.
- `task.rs`'s known gap — a shape result's slot tags come from the literal — is the same call-site
  recording; `crates/nvs-stdlib/src/task.rs:49`.
- Stage 4's two members and stage 5's `examples/input-shapes.nvs` — `docs/agent/loop-goal.md`.
