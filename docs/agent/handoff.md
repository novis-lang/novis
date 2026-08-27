# Handoff

## State

**M4 — language completeness.** A `static` property now has storage, read and write, end to
end: the checker records `ExprInfo::StaticProperty` (`crates/mwl-types/src/expr/mod.rs:265`),
`mwl_ir::ir::Program::statics` fixes one slot per declared static, `InstKind::StaticGet`/
`StaticSet` name it by `(declaring class, name)`, and `mwl-codegen` resolves that pair to an
index into a per-request `Value` vector reached through `mwl_runtime::STATICS_OFFSET` — two
loads and no call, the `FieldGet` shape with the context in place of a receiver.

**The storage is request-scoped**, armed by `Unit::install_in` and released by `Ctx`'s `Drop`.
`docs/adr/README.md` § *Decisions taken at project start* owns the decision (priority 1: a
process-global static is a channel from one request into the next); `mwl_runtime::ctx`'s
module docs own the mechanism and what it spends.

Two shapes are refused rather than answered wrongly: a non-nullable static with no
initializer is `E0409` at its declaration (no constructor can discharge an obligation on
storage that is not any instance's), and `static::$prop` is `E0499` — PHP re-resolves it
against the *called* class, which this slot layout cannot express. `E0499` is **the last
code in the `E04xx` band**; see the playbook.

`verify.py` 6 of 6 green — conformance **612**, differential **173**, 1633 unit tests.
`tools/leak-check.sh` clean over a fixture that assigns a `string` static from itself.

## Next group

**`exit` first, then late static binding for a static property.** They share no files; `exit`
is the item this session did not reach and is a feature that was never built, while the
late-binding hole is one this session created the diagnostic for and named the shape of.

- [ ] **`exit` and `exit(...)` need a distinguished unwind.** `ExprKind::Exit` is
      `crates/mwl-syntax/src/ast.rs:851`; it reaches `lower_expr_stmt`
      (`crates/mwl-ir/src/lower/stmt.rs:225`) and `lower_expr`'s catch-all
      (`crates/mwl-ir/src/lower/expr.rs:300`) with no arm. The status vocabulary is
      `crates/mwl-runtime/src/abi.rs:35` (`OK`/`THROWN`/`FATAL`) and the terminators are
      `crates/mwl-ir/src/ir.rs:1822` — decide whether `exit` is a fourth status or a
      `FATAL` carrying an exit code, and record it in `docs/adr/README.md`
      § *Decisions taken at project start*. `finally` must still run, which is what makes
      this an unwind rather than a `return`.
- [ ] **`static::$prop` could resolve like PHP instead of being refused.** It needs a
      per-class static slot table hanging off the `ClassDesc` rather than the flat
      unit-wide vector `mwl_ir::ir::Program::statics` is today
      (`crates/mwl-ir/src/ir.rs:22`), plus a runtime lookup keyed on the late-static-binding
      class the callee already carries in slot 0 (`crates/mwl-ir/src/lower/mod.rs:677`).
      Only a subclass that *redeclares* the static observes the difference. The refusal is
      `crates/mwl-types/src/expr/mod.rs:265` and its case is
      `tests/conformance/reject/a-static-property-is-refused-uninitialized-and-late-bound.mwlt`.

## Backlog

- The `E04xx` band is exhausted at `E0499` — the next type diagnostic needs a band decision
  (`docs/adr/README.md` § *Decisions taken at project start*).
- `ExprInfo::StaticProperty`'s write side goes through `lower_store`'s new arm
  (`crates/mwl-ir/src/lower/stmt.rs`); a compound `Class::$p += 1` was not exercised and
  may reach `lower_read_modify_write`'s staged-target path, which has no static arm.
- `docs/spec/02-php-migration.md` has not been re-scored since statics landed —
  `python tools/check-migration.py`.
- `holes.py` still reads 24 sites / 6 items: no worklist item named the static property, so
  the count is unchanged by this session.
