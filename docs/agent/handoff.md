# Handoff

## State

**Goal 18, stages 1 and 2 are complete; stage 3 has landed the fork, the carrier's runtime end and the
checker's recording half.** Optional shape fields, `tainted {…}`, guarded reads, the codec tables'
`required` column, `nvs_types::derive::shape_codec`, `nvs_runtime::ShapeCodec` and its `ClassTable`
owner all stand — `rule:types/shape-type`'s optional-field paragraph,
`rule:core-api/required-optional-and-nullable` and [ADR 0157](../decisions/0157.md) § 2 own that half,
and `crates/nvs-ir/src/lib.rs` § *Design choices* owns the fork (a shape's wire contract is a constant
of the call site, never of its class). No rule changed, so ADR 0159 is still unopened.

**What landed this session:** a call site writing an inline shape where the type-argument door expects a
class is now accepted and recorded twice — the synthesized class's label on
`ResolvedCall::written_shape`, and the per-field wire contract on the expression table under the **type
argument's own span**, since `{n: int}` and `{n: string}` share the label `$shape{n}`. The label's one
home is `nvs_types::derive::shape_class_label`; `nvs_ir::lower::shape_class_label` delegates to it.

**The emitting half is one slice behind the checker, and it panics rather than miscompiling.**
`nvs_ir::lower::written_class_label` (`crates/nvs-ir/src/lower/mod.rs:2983`) is the one reader for all
three call paths, and it panics naming the shape whose contract has no constant to ride in. So
`Core\Json::decodeAs<{n: int}>` type-checks today and ICEs at lowering — the next item is what removes
that, and the playbook bullet above dies with it.

The failing acceptance check (`examples/input-shapes.nvs`) is stage 5's fixture — an unwritten artefact,
not a regression.

## Next group

**Stage 3: the carrier is emitted and then consumed** — one file set:
`crates/nvs-ir/src/ir.rs`, `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-ir/src/print.rs`, `crates/nvs-codegen/src/lib.rs`, `crates/nvs-codegen/src/emit.rs`,
`crates/nvs-stdlib/src/arr.rs`, `crates/nvs-stdlib/src/registry.rs`. **In this order** — the coverage
gate fails the moment a registry row has no conformance case, so the member is last.

- [ ] **Emit the carrier, and register the shape's class with it** — a third constant beside the
      descriptor and the list flag at `crates/nvs-ir/src/lower/expr.rs:3217` and `:3434` (and
      `crates/nvs-ir/src/lower/closure.rs:623`), all three of which now read
      `written_class_label` (`crates/nvs-ir/src/lower/mod.rs:2983`) — deleting its shape panic is the
      slice's own acceptance. The contract is `self.exprs.shape_codec(span)` with the span
      `ResolvedCall::written_shape` carries, converted the way `codec_fields`
      (`crates/nvs-ir/src/lower/mod.rs:748` is its one caller) converts a class's, except that a shape
      field's `param` **is** its slot, so there is no layout to join against. It needs a
      `record_shape_class` (`crates/nvs-ir/src/lower/mod.rs:1793`) call so the descriptor constant
      resolves in a unit that spells that shape nowhere else, a new `InstKind` beside `ClassDescConst`
      (`crates/nvs-ir/src/ir.rs:507`, one arm each in `crates/nvs-ir/src/print.rs:179` and
      `crates/nvs-codegen/src/emit.rs:597`), and a program-global key for the codec, since the lists are
      collected per function and merged. `crates/nvs-codegen/src/lib.rs:1111` materializes it through
      `ClassTable::define_shape_codec` (`crates/nvs-runtime/src/object.rs:1335`) in the same second pass
      `link_codecs` runs in. `crates/nvs-ir/src/lib.rs` § *A shape's wire contract* is the design it
      implements.
- [ ] **`Core\Arr::shapeAs<T>` — the row, the roster entry and three cases in one slice** —
      `crates/nvs-stdlib/src/arr.rs` and the five edits
      ([conventions.md](conventions.md) § *A `Core` member*), plus the roster row at
      `crates/nvs-stdlib/src/registry.rs:2382`; the body reads its contract through
      `Value::as_shape_codec` (`crates/nvs-runtime/src/value.rs:588`) and hydrates with `as`'s own
      table. `docs/agent/loop-goal.md` § *Stage 3*.
- [ ] **`E_TYPE_ARG_NOT_A_CLASS` still says a class is the only answer** —
      `crates/nvs-types/src/expr/args.rs:1562`'s message and help predate the shape the door now takes,
      and `shapeAs` is the member that makes them misdirect. The expectation is frozen in
      `tests/conformance/core/db-query-as-builds-a-class-and-not-a-scalar.nvst:37`, so the case moves
      with the wording. `rule:types/arrays`'s type-argument sentence is amended by the slice above it.

## Backlog

- `rule:types/arrays`'s "a call site may write the type argument" sentence still names two members;
  stage 3 makes it three — `docs/agent/loop-goal.md` § *Stage 3* item 1.
- A shape written at `Core\Request::jsonAs`/`Core\Db\Queryable::queryAs` records neither a decode site
  nor a row site (`crates/nvs-types/src/expr/args.rs:1540`); both of those checks ask about a
  *declaration*, which a shape has none of — revisit when either member's body learns to hydrate one.
- Stage 4's `Core\Request::postAs`/`queryAs`, then stage 5's proofs, including
  `examples/input-shapes.nvs` — `docs/agent/loop-goal.md`.
