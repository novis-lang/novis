# Handoff

## State

**Goal 18, stage 2's checker half is complete.** A shape field carries a required bit, `{name?: T}`
writes it, assignability lets an optional key be absent, `tainted {…}` distributes, and now every
subscript **and** shape-property level of a `??`, `isset` or `empty` operand is marked guarded by one
shared walk (`crates/nvs-types/src/expr/presence.rs:91`), with a guarded read of an *optional* field
answering `?T` and recording `guarded` on its `ExprInfo::ShapeProperty` entry.
[ADR 0157](../decisions/0157.md) is the goal's one record; § 2 is what the above implements.

**The lowering half of that is not landed, and nothing about run-time behaviour changed this
session.** `InstKind::SlotGet` has exactly one absence answer — the throw — where `InstKind::ArrayGet`
carries an `AbsentKey`, so `$p->a ?? 0` and `isset($p->a)` still throw on an absent key.
`crates/nvs-ir/src/lower/expr.rs:3804` is the site, and its comment says so. That is the next group.

Stages 3 (the converter) and 4 (the two request members) have not started. The failing acceptance
check (`examples/input-shapes.nvs`) is stage 5's fixture and is still an unwritten artefact, not a
regression.

## Next group

**Stage 2: the lowering the guarded bit is waiting for** — one file set:
`crates/nvs-runtime/src/object.rs`, `crates/nvs-ir/src/ir.rs`, `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-codegen/src/emit.rs`, `tests/conformance/lang/`.

- [ ] **A runtime entry point answers `null` for a name the class does not carry** — a
      `nvs_object_slot_optional_get` beside `crates/nvs-runtime/src/object.rs:2986`, which is
      `nvs_object_slot_get` with the missing-name throw replaced by a `null` and every other throw
      (non-object receiver, `Tag::Unset` slot) left exactly as it is. Registered in the symbol table
      at `crates/nvs-runtime/src/helpers.rs:2918` and re-exported at
      `crates/nvs-runtime/src/lib.rs:360`. `rule:types/shape-type`'s optional-field paragraph is the
      specification and ADR 0157 § 2 the reasoning.
- [ ] **`InstKind::SlotGet` carries an absence answer, and the guarded read takes it** —
      `crates/nvs-ir/src/ir.rs:736` gains an `absent: AbsentKey` (`crates/nvs-ir/src/ir.rs:1523`),
      `crates/nvs-ir/src/lower/expr.rs:4172` fills it from `ExprInfo::ShapeProperty::guarded` and
      takes `Ty::Tagged` for the `Null` case exactly as `crates/nvs-ir/src/lower/expr.rs:4606` does
      for a subscript, and `crates/nvs-codegen/src/emit.rs:2510` picks the symbol name off it —
      the C signature is unchanged, so `RuntimeSig::SlotGet` is reused rather than duplicated.
      `crates/nvs-ir/src/print.rs:219` and the `InstKind::SlotGet` match at
      `crates/nvs-codegen/src/emit.rs:636` are the other two sites.
- [ ] **One conformance case pins both halves** — a new case beside
      `tests/conformance/lang/a-shape-read-through-a-widened-view-is-name-keyed.nvst:1`:
      `$p->a ?? "d"` over a `{a?: string}` value built without `a` prints the default, and the same
      read outside a `??` throws catchably. What it exercises is
      `crates/nvs-ir/src/lower/expr.rs:4172`, so it is the slice above's proof rather than a second
      one. `rule:types/shape-type` is what it cites. Never an `--ORACLE--` section there
      (`docs/agent/conventions.md` § *A `.nvst` test case*).

## Backlog

- A guarded property level whose *base* is itself a guarded read is `E_NULLABLE_RECEIVER` today —
  `crates/nvs-types/src/expr/mod.rs:583` drops the base's `null` for a subscript chain and the
  property arm has no counterpart, so `$p->a->b ?? 0` over a nested optional shape is refused. Doing
  the same there would also make `$obj->x ?? 0` legal on a `?Foo`, which is PHP's behaviour but no
  rule states it — needs a paragraph in `rule:types/shape-type` or a `changes:` on ADR 0157 first.
- An erased receiver (`object`, `mixed`) now records `guarded: true` under `??`/`isset`/`empty`, so
  the lowering group above also closes `isset($m->x)` throwing where PHP answers `false`. Worth a
  line in `rule:types/erased-member-access` once it runs.
- Stage 3, the general `as`-shaped converter, has not started.
- Stage 4, the two request-boundary wrappers, has not started.
- `examples/input-shapes.nvs` is stage 5's acceptance fixture and is unwritten.
