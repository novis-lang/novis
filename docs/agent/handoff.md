# Handoff

## State

**An anonymous shape value constructs.** `{x: 1, y: 2}` lowers to one `InstKind::New` of a
compiler-synthesized class plus one `FieldSet` per field (`expr.rs`'s `lower_object_literal`). The
class is keyed on the **sorted** field names, not on the site — `lower::shape_class_label` renders
`$shape{x,y}` — so every literal of one shape shares one `ir::Class`, and its slot numbering is the
same one `ExprInfo::ShapeProperty` hands the read side. It travels out of a body through
`Lowering::shapes`, beside `closures`, because only `lower_file` can hold a class; that file dedups
by label. Nothing was added to codegen: the synthesized class goes through `Classes::define` like
any other.

Verify is green (1539 tests, clippy and fmt clean). Conformance is **429** — the new case is
`tests/conformance/lang/an-anonymous-object-literal-constructs-and-reads-its-fields.mwlt`;
differential 89. Valgrind is clean over the new refcount edges (200 iterations of a literal owning a
retained aliasing read, a fresh producer's result and a nested shape object).

**A defect this slice made reachable, and the next group's first job.** A fixed-offset `SlotGet` is
only right where the receiver's static shape is the value's *own* shape. Through a **widened view**
— ADR 0036 § 3's width subtyping, or a named class satisfying a shape — it reads the wrong field:
a `{y: int}` parameter handed `{x: 1, y: 2}` answers `1`. That is exactly why § 4 specifies a
name-keyed fetch rather than an offset. The class-receiver half predates this session's work.
`mwl-ir` crate-doc gap 6 and `mwl_types::expr::members`' shape arm both say so; nothing in the
corpus writes a widened view, so no test asserts the wrong answer.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, which is the gate's frontier.

## Next group — shape values, which is one file set

**Shared file set:** `crates/mwl-ir/src/lower/expr.rs` (`lower_object_literal` at `expr.rs:2975`,
`lower_shape_property_access` at `expr.rs:3026`), `crates/mwl-ir/src/lower/stmt.rs:416` (the
assignment arm's `ExprInfo` match), `crates/mwl-ir/src/ir.rs:505` (`InstKind::SlotGet`),
`crates/mwl-codegen/src/emit.rs:1462` (`emit_slot_get`) and
`crates/mwl-types/src/expr/members.rs:386` (the shape arm that records the slot).

- [ ] **1. A shape read through a widened view is name-keyed, not offset-keyed.** ADR 0036 § 4 in
      full. The runtime class table carries no field *names* today
      (`mwl-codegen/src/lib.rs`'s `Classes::define` passes `fields.len()` alone to
      `mwl_runtime::ClassTable::define`), so the shape is: names on `ClassDesc`, one helper that
      answers a slot by name, and `SlotGet` carrying the name beside the index. § 4 also makes a
      missing name a catchable throw, so the instruction becomes fallible with a landing block.
      Weigh keeping the offset fast path where the receiver's type is a literal's own shape.
- [ ] **2. A shape field write.** `stmt.rs:417`'s assignment arm takes `ExprInfo::Property` /
      `ExprInfo::Index` and panics for `ExprInfo::ShapeProperty`; it needs the `SlotSet` counterpart
      of `SlotGet` (release the old value, store the new), and slice 1 decides how it addresses the
      slot, so do it second.
- [ ] **3. `{a: 1}` as a statement-level expression and out of an arrow body.** ADR 0036 § 2's
      grammar note — both need `({...})`, and `E0117` already says so; a conformance case pinning
      the two spellings costs one file and is independent of 1 and 2.

## Backlog

- `Core\Out::capture` — the gate's frontier fixture (`examples/collect.mwl:47`); M4S sink work.
- A repeated field name in one literal (`{a: 1, a: 2}`) is undiagnosed; `mwl-ir` panics on it
  rather than lowering — `mwl_types::expr` owns the missing check.
- `do`/`while` does not lower (`mwl-ir` gap 1's remainder).
- ADR 0088's qualifier classification for every `mwl-stdlib` member row.
- ADR 0071's decode gaps: no enum/`decimal`/`Instant`/`array`/nested-class field.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
