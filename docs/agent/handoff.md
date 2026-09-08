# Handoff

## State

**Goal 18, stage 2 is complete.** A shape field carries a required bit, `{name?: T}` writes it,
assignability lets an optional key be absent, and `tainted {…}` distributes over a shape's
text-carrying fields at parse time. [ADR 0157](../decisions/0157.md) is the goal's one record and it
covers the stage whole, including the `tainted mixed` question — refused, and goal 16's prose
spelling is what gets corrected. Stages 3 (the converter) and 4 (the two request members) have not
started.

**One thing the record decided is ahead of the tree.** ADR 0157 § 2 settles that a read of an
optional field answers the declared `T` and an absent key is `rule:types/erased-member-access`'s
catchable throw, with `??` and `isset` as the spellings that ask without throwing. The guarded half
is not implemented — `Env::coalesce_guarded` marks `Index` levels only — so `$p->a ?? 0` still
throws instead of answering the default. That is the next group.

The failing acceptance check (`examples/input-shapes.nvs`) is stage 5's fixture and is still an
unwritten artefact, not a regression.

## Next group

**Stage 2: the guarded read the record commits to** — one file set:
`crates/nvs-types/src/expr/presence.rs`, `crates/nvs-types/src/expr/mod.rs`,
`crates/nvs-types/src/expr/members.rs`, `tests/conformance/lang/`.

- [ ] **A shape-property level of an `isset`/`??` operand is marked guarded** —
      `crates/nvs-types/src/expr/presence.rs:97` and `crates/nvs-types/src/expr/mod.rs:331` are the
      two places that fill `Env::coalesce_guarded` with `Index` levels today, and a property level
      joins them. `rule:types/shape-type`'s optional-field paragraph is the specification and ADR
      0157 § 2 the reasoning.
- [ ] **A guarded optional read answers `?T` and an unguarded one still answers `T`** —
      `crates/nvs-types/src/expr/members.rs:1194`, where a shape field's declared type is recovered;
      `crates/nvs-types/src/expr/mod.rs:576` shows how the `Index` arm reads the same mark. The
      runtime half is `nvs_ir::ir::InstKind::SlotGet` having to answer absent rather than throw when
      the fetch is guarded.
- [ ] **One conformance case pins both halves** — a new case beside
      `tests/conformance/lang/a-shape-read-through-a-widened-view-is-name-keyed.nvst:1`, where an
      absent optional field throws on a bare read and answers the default under `??`
      (`rule:types/shape-type`, `rule:types/erased-member-access`).

## Backlog

- Stage 3's converter, `Core\Arr::shapeAs<T>` — `crates/nvs-stdlib/src/arr.rs`, goal § Stage 3.
- Stage 4's `postAs<T>` and `queryAs<T>` — goal § Stage 4.
- `examples/input-shapes.nvs`, the acceptance fixture — goal § Stage 5.
- `rule:core-api/shape-rules` R15's worked list and spec §§ 6 and 15's rosters gain the optional
  marker — goal § Standing decisions.
