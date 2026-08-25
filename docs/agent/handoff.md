# Handoff

## State

**A shape field read is name-keyed, so a widened view answers the field it names.** ADR 0036 § 4's
fetch is one call to `mwl_runtime::mwl_object_slot_get` (`object.rs:1571`), which resolves the name
on the receiver's *own* descriptor: `ClassDesc` now carries `fields: Vec<String>` in slot order and
`ClassDesc::field_slot(name, hint)` (`object.rs:363`) tries the hint first, then scans. The hint is
the slot the receiver's *static* shape gave, so it hits on every unwidened read and is simply wrong
through a `{y: int}` view of a `{x: 1, y: 2}` — which is the bug this closed. `ClassTable::define`
takes the field *names* now rather than a count; the count is `fields.len()`, so the two cannot
disagree. `InstKind::SlotGet` carries the name beside the hint and is **fallible** (a name the
concrete class lacks is § 4's catchable throw), so every shape read has a landing block. That costs
a call where there used to be one inline load; § 4 mandates exactly this and defers the
per-call-site specialization to its own *Revisiting*, so the offset fast path was not kept.

Verify is green (1539 tests, clippy and fmt clean). Conformance is **430** — the new case is
`tests/conformance/lang/a-shape-read-through-a-widened-view-is-name-keyed.mwlt`; differential 89.
Valgrind is clean over the new landing block (200 iterations of a widened read off a fresh producer
and off a held literal, each with a refcounted field).

**A named class does not satisfy a shape type** — `E0401`, checked this session. Width subtyping is
shape-to-shape only, so that is the only widening; the doc comments claiming otherwise are fixed and
`playbook.md` says so.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, which is the gate's frontier.

## Next group — shape values, still one file set

**Shared file set:** `crates/mwl-ir/src/lower/expr.rs` (`ShapeField` at `expr.rs:14`,
`lower_object_literal` at `expr.rs:2993`, `lower_shape_property_access` at `expr.rs:3052`),
`crates/mwl-ir/src/lower/stmt.rs:417` (the assignment arm's `ExprInfo` match),
`crates/mwl-ir/src/ir.rs:525` (`InstKind::SlotGet`), `crates/mwl-codegen/src/emit.rs:1472`
(`emit_slot_get`), `crates/mwl-runtime/src/object.rs:1571` (`mwl_object_slot_get`) and
`crates/mwl-types/src/expr/members.rs:383` / `expr_table.rs:262` (`ExprInfo::ShapeProperty`).

- [ ] **1. A shape field write.** ADR 0036 § 4's write half, which is this session's read mirrored:
      a `SlotSet` keyed on the name with the same hint, the same missing-name throw, and *no* new
      field ever created. `stmt.rs:417`'s arm matches only `ExprInfo::Property`/`HookedProperty`, so
      a shape target falls through to the panic below it. **One thing the read did not have to
      answer:** § 4 also wants the incoming value checked against the field's *real* declared type,
      and `ClassDesc` carries names but no field types. Decide it — either add types beside
      `fields`, or check the runtime tag alone and record what that does not catch in
      `mwl-runtime`'s module doc.
- [ ] **2. `{a: 1}` as a statement-level expression and out of an arrow body.** ADR 0036 § 2.
      `lower_object_literal` (`expr.rs:2993`) works from every expression position that reaches it;
      what to check is the four exits a synthesized class has (`playbook.md`'s own bullet) and
      whether a statement-position literal is dropped without releasing its fields.
- [ ] **3. An *erased* receiver's read — `$o->x` on a plain `object`.** `mwl-ir` gap 6's remaining
      half, and it is newly cheap: `SlotGet` is already the name-keyed fallible fetch § 4 asks for,
      so what is missing is only `mwl_types::expr::members` recording an entry for the erased case
      (`members.rs`'s `Ty::Object => return env.interner.mixed()` arm) instead of nothing.

## Backlog

- `Core\Out::capture` — § 12's last member, gated behind M4S's sink work (`docs/implementation-plan.md`).
- `Core\Json::decodeAs<T>` — `mwl_stdlib::json` gap 2; a written call-site type argument exists now.
- ADR 0088's qualifier classification on every `mwl-stdlib` registry row (plan, *Open now*).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADR 0071's non-scalar decode: enum, `decimal`, `Instant`, `array` and nested-class fields.
