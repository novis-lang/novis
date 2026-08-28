# Handoff

## State

**M4's frontier is Stage 5's remainder: items 33 and 36 of `docs/agent/loop-goal.md`, plus item
39's last gap.** Item 39's two signature-level gaps landed this session — the promoted-parameter
backlog line and the implicit constructor's arity — and ADR 0043 § 4 now has no open bullet at all.

- **The mechanism has one home each and is not restated here**: which parameters promote is
  `nvs_syntax::ast::Param::is_promoted` (`crates/nvs-syntax/src/ast.rs:526`), asked in both
  directions by `nvs_types::signatures` — `record_promoted_properties`
  (`crates/nvs-types/src/signatures.rs:882`) for a `constructor` and
  `reject_promotion_outside_constructor` (`crates/nvs-types/src/signatures.rs:902`, `E0722`) for
  every other method — and the implicit constructor's arity is
  `reject_arguments_to_implicit_constructor` (`crates/nvs-types/src/expr/calls.rs:293`, `E0402`).
- **`nvs-types`' known-gaps list is the worklist for what is left of item 39**
  (`crates/nvs-types/src/lib.rs:197`): one entry survives, the `foreach` **key** binding, and it is
  the next group's first slice.
- **`orient.py`'s pack was complete for this item.** `[context] modules` still prints no map line
  for `nvs-types`' `signatures`, `expr/calls` or `lib`, all of which this session edited, and
  `[context] adrs` has no ADR 0043 § 4 entry — the section both slices are written against.

## Next group

**Item 39's last gap and the two Stage 5 items that share the checker's expression tree.** The file
set is `crates/nvs-types/src/expr/iteration.rs`, `crates/nvs-types/src/lib.rs` (the known-gaps list
each slice deletes its own line from) and `tests/conformance/lang/`.

- [ ] **A `foreach` *key* binding declared at anything but `string` is refused**, ADR 0007 § 5:
      an array has one stored key type, so `foreach ($a as int $k => …)` is always wrong and today
      type-checks and then trips `nvs_ir`'s assertion instead. The check belongs beside
      `report_not_iterable` (`crates/nvs-types/src/expr/iteration.rs`, `python tools/peek.py
      --locate report_not_iterable`); the known-gap line is `crates/nvs-types/src/lib.rs:197` and
      the next free code is `E0723`.
- [ ] **Item 33**, `docs/agent/loop-goal.md` — read its own line before starting; it was not in
      this session's pack.
- [ ] **Item 36**, same.

## Backlog

- A `require` whose path is not a string literal runs nothing at all, silently, in both forms —
  `nvs_hir::requires`' own known gap.
- ADR 0043 § 4's `E_DELEGATE_TYPE_MISMATCH` "does `$field`'s type satisfy this interface" check is
  built; what is still not is the whole-class conformance story for a delegated interface whose
  field is itself refused — `nvs_types::conformance`'s own doc comment owns it.
- `docs/spec/02-php-migration.md`'s score, `python tools/check-migration.py`.
- The named `.nvst` cases each stage still owes, `python tools/loop.py --list`.
