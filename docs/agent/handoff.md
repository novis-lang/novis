# Handoff

## State

**ADR 0135's checker half is on disk.** `Ty::Options` is now
`Ty::CoreShape(Vec<CoreShapeField>)` — a name, a `TypeId` and a `required` flag per key — and both
registry spellings intern into it through `nvs_types::core_lib`: `CoreTy::Options` as an all-optional
field list through `TypeInterner::options`, `CoreTy::Shape` as § 3's **merged, name-deduplicated** list
through `merge_shape_arms` + `TypeInterner::core_shape`. Behaviour for every existing bag is unchanged
(every field optional, same describe, same checks); the only new test is
`a_shapes_arms_merge_in_declaration_order_and_deduplicate_by_name`, which holds § 3 over a two-arm shape
because no registry row declares one yet.

**Two halves of 0135 are still missing, and they are different in kind.** First, `nvs-ir` cannot lower a
*required* shape parameter: `lower_options_arg` already iterates the merged fields correctly, but the
per-field defaults come from the parameter's own `ConstArg::Options` entry, which `core_lib::defaults_of`
synthesizes only for the trailing bag — and synthesizing one for a shape would make `MethodSig::required`
treat the parameter as optional. Second, § 2's *exactly one arm accepts it* cannot be stated on a merged
list at all; `Ty::CoreShape`'s own doc carries that as its known gap. `Core\Db::open` needs both, so the
goal's one open acceptance check, `a_db_open_target_in_a_denied_range_fails`, still does not run — an item
still open, not a regression.

**`orient.py` again printed no section of ADR 0135**, and the whole item was §§ 1, 3 and 4. `[context]
adrs` in `docs/agent/loop-goal.toml` names 0133 and 0067 only; add `0135` with §§ 1-4. This is the second
session to pay one peek for it.

## Next group

**0135's IR half, then the check, then the member. File set: `crates/nvs-types/src/core_lib.rs` with
`crates/nvs-ir/src/lower/mod.rs` and `crates/nvs-ir/src/lower/call.rs`, then
`crates/nvs-types/src/expr/args.rs` with `crates/nvs-diagnostics/src/lib.rs`, then
`crates/nvs-stdlib/src/db.rs`.** The first two items share one build; the third is its own session.

- [ ] **Record a shape parameter's per-field defaults without making the parameter optional, and flatten
      one at the call site** (0135 § 3). `crates/nvs-types/src/core_lib.rs:226` is `defaults_of`, whose
      `index == positional` branch is the only route to a `ConstArg::Options` entry today;
      `crates/nvs-ir/src/lower/mod.rs:2899` is `options_defaults`, which panics on anything else, and
      `crates/nvs-ir/src/lower/call.rs:198` is the caller. § 3's fill rule is "the field's own default
      where it has one, `Const::Null` for a field of an arm the caller did not write", so the defaults
      list is a property of the *type*, not of the parameter's optionality — a second `ConstArg` variant,
      or a slot beside `MethodSig::defaults`, is the call.
- [ ] **Refuse a missing required key and a key no arm declares, and widen `E0453`/`E0454` from "option"
      to "shape key"** (0135 §§ 2 and 5). `crates/nvs-types/src/expr/args.rs:596` is `check_options_arg`,
      which already reports both codes and now has `required` to read;
      `crates/nvs-diagnostics/src/lib.rs:830` and `:835` are the two codes, one clause each and **no new
      code** — both type bands are full. Arm selection needs `crates/nvs-types/src/ty.rs:286` widened to
      carry the arms, or the equivalent per-field masks.
- [ ] **`Core\Db::open`'s five edits, and `a_db_open_target_in_a_denied_range_fails` with them**
      (0067 § 3, 0135 § 3). `crates/nvs-stdlib/src/db.rs:506` is `connect`'s row and the block the
      `Db\Settings` arms go beside; `crates/nvs-stdlib/src/db.rs:66` is the known gap to delete.

## Backlog

- Add `0135` §§ 1-4 to `[context] adrs` in `docs/agent/loop-goal.toml` — two sessions have paid for it.
- `ParamDoc::shape` and `a_documented_rows_param_docs_agree_with_its_names` extend to a shape's merged
  key set — ADR 0135 § 4.
- `every_member_parameter_carries_a_qualifier_classification` must walk into a shape's fields — ADR
  0135 § 3; `host` is `Qual::Sink` on the field, not on the parameter.
- MySQL sends only `query` of the four members that send — `nvs_stdlib::db` known gap 2.
- Three drivers have no connect path — `nvs_stdlib::db` known gap 2.
