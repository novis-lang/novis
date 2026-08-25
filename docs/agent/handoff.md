# Handoff

## State

**ADR 0036 is whole, erased receiver included.** `$o->x` on a plain `object` reads *and* writes: the
checker records an `ExprInfo::ShapeProperty` carrying the written name alone — slot `0`, type `mixed`
— for both the plain-`object` receiver and a name a shape does not list, and `mwl-ir` lowers each to
the `SlotGet`/`SlotSet` a shape field already used. § 4's missing-name throw is reachable at last and
is pinned by
`tests/conformance/lang/a-property-access-through-a-plain-object-receiver-is-name-keyed.mwlt`.
`mwl-ir` gap 6 is closed for property access; array access is still compile-time-target-only.

**The erased write reaches *named* classes, which made per-slot field tags mandatory rather than
optional.** Without them `$o->count = "s"` through an `object` view would have left a `string` in a
slot the class declares `int`. So `mwl_types::check::record_property_types` copies every class's own
declared property types into the expression table, and `mwl_ir::lower`'s `field_reprs` joins them
against the flattened slot order the same way `property_defaults` already does — own class first,
then ancestors, an unclaimed slot falling back to `Ty::Tagged` (= unchecked) so the vector stays
index-aligned. `mwl_runtime::object`'s module doc § *What a shape write checks* owns what a tag
catches and its five gaps; that section and `ir::Class::field_reprs` are the two homes.

Verify is green (1540 tests, clippy and fmt clean). Conformance **433**, differential 89. Valgrind
clean over the new erased edges — read out of a slot, a fresh producer written through an erased
view, self-assignment, and both missing-name throw edges — 200 iterations.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, and that member is genuinely blocked
behind ADR 0088's sink carriers (`Core\Html\Markup`/`Cli\Text`), which spec § 12's own prose makes
its return type. It is M4S work, not a slice to open ahead of the sinks.

## Next group — M4's three remaining operator/control-flow holes (`mwl-ir` gap 16, `lib.rs:266`)

These are named in the goal's standing decisions as in scope precisely because the corpus cannot be
written around them, and Stage 4's counts (433 of 600) are now the gate's own work.

**Shared file set:** `crates/mwl-ir/src/ir.rs:1380` (`BinOp`),
`crates/mwl-ir/src/lower/expr.rs:161` (the binary-operator match),
`crates/mwl-codegen/src/emit.rs:874` (`emit_binop`'s representation rows),
`crates/mwl-ir/src/lower/stmt.rs:166` (`StmtKind::While`'s arm) and
`crates/mwl-ir/src/lower/control.rs:102` (`lower_while`).

- [ ] **1. The bitwise and `**` binary operators lower.** `ir::BinOp` stops at the arithmetic,
      equality and ordering rows, so `&`, `|`, `^`, `<<`, `>>` and `**` panic in `lower_expr`. Each
      needs a `BinOp` variant and an `emit_binop` row over `Int | Uint`. This also lands `&=`, `|=`,
      `^=`, `<<=`, `>>=` and `**=` for nothing: `lower_compound_assignment` already rewrites
      `$x op= e` into `$x = $x op e`, so a compound form arrives the moment its binary form does.
- [ ] **2. `$x++`, `$x--`, `++$x` and `--$x` lower.** Same rewrite shape as a compound assignment
      and the same re-evaluation rule — `is_reevaluable_target` is what refuses `f()->count += 1`,
      and an increment inherits it. Watch the postfix/prefix *value* difference, which the compound
      form has no analogue for.
- [ ] **3. `do { … } while (…);` lowers.** The one M4 control-flow statement that does not; every
      terminator it needs exists, and it is `lower_while` with the body block entered before the
      test rather than after.

## Backlog

- `Core\Out::capture` — the last key in `spec-members-outstanding.txt`; blocked on ADR 0088's sinks.
- ADR 0092 (one diagnostic record, three renderings) and 0091 (run modes) — plan § *Open now*.
- A promoted constructor parameter still claims no slot (`mwl_types::layout`'s own module doc § 38),
  so `$obj->x` on one is `E0405`. PHP writes them everywhere; the differential corpus will meet it.
- `Core\Json::decodeAs<T>`'s wider codec-reachable field set — `mwl_stdlib::json`'s gaps.
- Nullsafe assignment target (`$a?->b = v`) panics rather than being diagnosed — `mwl-ir` gap 6.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.

## Orientation gaps found this session

`[context]` in `loop-goal.toml` did not select **ADR 0036 § 4** (the section the whole goal item was
about — sliced by hand) and names no `mwl-ir` module pattern at all, so neither `mwl-ir/src/lib.rs`'s
gap list nor `lower/*` appeared in the map. Both are worth adding before the next group, which lives
entirely in `mwl-ir` and `mwl-codegen`.
