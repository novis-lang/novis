# Handoff

## State

**Spec § 10 is whole: a shape's field is readable.** `$issue->path` type-checks to a *slot index* —
`ExprInfo::ShapeProperty` (`expr_table.rs`), resolved at `members.rs:363`'s shape arm — and `mwl-ir`
reads that slot by number through the new `InstKind::SlotGet`, so no class label and no layout table
is involved at all. The two sides agree because a shape's field list is sorted when it is interned
(`TypeInterner::shape`) and every producer lays its slots out that way; `mwl_stdlib::issue`'s `FIELDS`
is that agreement written down, and its module doc now says what reordering it would cost. The other
half of the slice was **`erase_checked_ty` gaining a `Ty::Shape` arm** — a shape value is an ordinary
refcounted instance, so its representation is `Ty::Object`, which is what makes a shape-typed local,
`foreach` binding, array element and property lower at all.

Verify is green (1539 tests, clippy and fmt clean). Valgrind is clean over the new read edge — 200
iterations of a failed `Core\Json::decode` and a failed `decodeAs<Row>`, reading `path`/`message` off
every issue by index and by `foreach`. Conformance is **428** (two existing cases grew; no new file),
differential 89. `examples/collect.mwl` still exits 1 at `Core\Out::capture`, which is the frontier.

**Still not lowered, and the next group's work:** an anonymous `{a: 1}` literal (`expr.rs:230` panics
naming `ObjectLiteral`) and a shape field *write* (`stmt.rs:417`). `mwl-ir`'s crate-doc gap 6 states
both.

## Next group — an anonymous shape value, which is one file set

**Shared file set:** `crates/mwl-ir/src/lower/expr.rs` (the `ObjectLiteral` arm at `expr.rs:230`,
beside `lower_shape_property_access` at `expr.rs:2944`), `crates/mwl-ir/src/ir.rs` (`Program.classes`
at `ir.rs:19`, `Class` at `ir.rs:32`, `InstKind::SlotGet` at `ir.rs:505`) and
`crates/mwl-codegen/src/lib.rs` (`Classes::define`, ~`lib.rs:446`). ADR 0036 § 2 specifies the value;
§ 4 is the read that already works.

- [ ] **1. `{x: 1, y: 2}` constructs.** ADR 0036 § 2 makes each literal a *compiler-synthesized*
      class — methodless, no constructor, ordinary reference semantics — so the cheapest shape is for
      lowering to append one `ir::Class` per distinct shape to `Program.classes` (fields in **sorted**
      name order, so the slot numbering `ExprInfo::ShapeProperty` already hands out is the same one)
      and emit the `New` + `FieldSet` sequence that exists. Nothing new is needed in codegen if the
      synthesized class goes through `Classes::define` like any other. `mwl_types::expr::mod.rs:158`
      is where the literal is typed today, so the checker half is done.
- [ ] **2. A shape field write.** `stmt.rs:417`'s assignment arm takes `ExprInfo::Property` /
      `HookedProperty` only and panics for a `ShapeProperty` target; the symmetric `InstKind::SlotSet`
      is a copy of `SlotGet` plus the release-the-previous-value pair that arm already emits for a
      class field. Only reachable once slice 1 lands — every shape value today is `Core`-built and
      never written to.
- [ ] **3. The `tests/conformance/lang/` case.** ADR 0036 §§ 2 and 4 pinned on the language's own
      receiver rather than only through `Core\Issue`: a literal, a field read, a field write, and the
      parenthesis rule § 2's grammar note states (a statement-initial `{` is a block, so a discarded
      literal is written `({a: 1})`).

## Backlog

- `Core\Out::capture` — the last `§12` key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`,
  and what `examples/collect.mwl` stops on; it lands with M4S's sink work (ADR 0092).
- `Core\Json::decodeAs<T>` still owes ADR 0071 § 2's wider codec set and § 4's default-bearing rows —
  `mwl_stdlib::json`'s own known gaps.
- `$e->previous` cannot be read *through* without a `!= null` narrowing first — `mwl_ir::Ty::Tagged`'s
  known gap, stated in `mwl_types::error_lib`'s module doc.
- Stage 4's counts are their own work: conformance 428 of 600, differential 89 of 150
  (`docs/implementation-plan.md`, *Open now*).
- `do`/`while` does not lower; an abandoned generator skips its `finally` (`mwl-ir` gaps).
