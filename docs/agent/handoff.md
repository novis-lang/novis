# Handoff

## State

**ADR 0036 is built except for the plain-`object` receiver.** A shape field is read *and* written by
name (`InstKind::SlotGet`/`SlotSet` → `mwl_runtime::mwl_object_slot_get`/`_slot_set`), so both are
right through a widened view, and a literal lowers in every position § 2 spells — including
discarded as `({a: 1});` and returned from an arrow body as `fn() => ({a: 1})`. What is left of
`mwl-ir` gap 6 is exactly the erased receiver: `$o->x` on a plain `object` records no `ExprInfo`
and panics, read or write.

**The write's type check is tag-granular, and that was the session's one design call.** § 4 wants
the incoming value checked against the field's *real* declared type, because § 3 compares shape
field types by ordinary assignability while a shape value is aliased — `{n: int|string}` is a legal
view of a `{n: int}` value. The declared type cannot live on the descriptor: a shape class is named
for its field *names* alone (`$shape{x}`), so `{x: 1}` and `{x: "s"}` are one class. `ClassDesc`
therefore carries one `Tag` per slot, or `None` where the type admits several — closing
representation confusion and leaving class identity and array element types uncaught.
`mwl_runtime::object`'s module doc § *What a shape write checks* is the home of that decision and
its five named gaps; do not re-open it without reading that section.

Verify is green (1540 tests, clippy and fmt clean). Conformance **432**, differential 89. Valgrind
clean over the new write edges — fresh producer, aliasing read, self-assignment, the refusing
throw edge, and the discarded literal — 200 iterations each.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, which is the gate's frontier.

## Next group — the erased receiver, closing `mwl-ir` gap 6

**Shared file set:** `crates/mwl-types/src/expr/members.rs:393` (the `Ty::Object => mixed` arm that
records nothing) and `expr_table.rs:262` (`ExprInfo::ShapeProperty`),
`crates/mwl-ir/src/lower/expr.rs` (`lower_shape_property_access` at `:3065`,
`lower_shape_property_assign` just below it), `crates/mwl-ir/src/lower/stmt.rs:441`'s
`ShapeProperty` arm, `crates/mwl-ir/src/ir.rs` (`SlotGet`/`SlotSet`),
`crates/mwl-ir/src/lower/mod.rs:488` (`ir::Class` from `layout`), `crates/mwl-codegen/src/lib.rs:483`
(`set_field_tags`).

- [ ] **1. `$o->x` on a plain `object` reads.** ADR 0036 § 4's erased half. The checker's
      `Ty::Object` arm returns `mixed` and records nothing; record an `ExprInfo` for it (a new
      variant, or `ShapeProperty` with no slot to hint) so `SlotGet` lowers at `Ty::Tagged` with a
      hint of 0. The missing-name throw already exists and is currently unreachable — this is what
      reaches it, so a `.mwlt` case can finally pin it.
- [ ] **2. The erased *write*.** Same `ExprInfo`, straight into the `SlotSet` that now exists —
      `stmt.rs`'s shape arm needs only to accept the new variant. Nothing else changes.
- [ ] **3. Field tags for a named class.** `ir::Class::field_reprs` is empty for every class
      `lower/mod.rs:488` builds, so slice 2's write into an ordinary object is unchecked. Fill it
      from `mwl_types::layout`'s declared field types through the same `erase_checked_ty` map, and
      strike gap 5 from `mwl-runtime`'s § *What a shape write checks*.

## Backlog

- `Core\Out::capture` — the last `§12` key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`.
- `Core\Json::decodeAs<T>` — `mwl_stdlib::json` gap 2; a written call-site type argument exists now.
- Calling a closure held in a local (`$f()`) does not lower at all — `mwl-ir`, `Call` with a
  `Variable` callee. Blocks observing any closure result without a `Core` member in the middle.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
- ADR 0088's qualifier classification on `mwl-stdlib` member rows — see the plan's *Open now*.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
