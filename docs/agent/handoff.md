# Handoff

## State

**M4 — ADR 0066 § 3's `as ?T` table has no text row left, and the object half of `as string` is
two thirds closed.** An operand whose static type names a class still resolves its `toString` in
`mwl_types` and lowers to an ordinary call; one that names *none* is dispatched on its runtime
class by `mwl_runtime::stringify`, which is the one home for that half of ADR 0028 § 1.

- **`echo $o`, `"" . $o` and `$o as string` are one answer for one value.** `concat_operand`
  already fell back to `Helper::TaggedToString` for an unresolved object; `convert`'s
  `(Ty::Object, Ty::Str)` row panicked instead, and now takes the same fallback. Under it,
  `stringify` asks the receiver's class for `toString` and `value_to_string` is the tag table
  beneath, so a class with none throws catchably and names itself.
- **The `Core`-owned class is the last third and is a `mwl-stdlib` question, not an `mwl-ir` one.**
  Its members are native, so nothing is in the compiled method table for that dispatch to find;
  `mwl-ir`'s known gap 12 is the home, and the standing decision names `mwl_stdlib::registry` as
  where "which `Core` classes have a `toString`" is stated.
- **`Helper::ToStringOrNull` shares `TaggedToString`'s implementation**, so the two spellings
  cannot disagree on a row. It is the one `?` helper emitted through `emit_fallible`: a
  `toString()` body that throws is the program's own exception and propagates — see the playbook
  bullet, which is what that cost.
- **The item this session was handed was already closed.** A `Stringable`-typed receiver resolves
  and lowers today; `implements_interface`'s self-pair answer closed it. The panic's reachable
  shapes were the erased `object` and the `Core` class, and `holes.py` should be re-read rather
  than the item's own prose.

## Next group

**The conversion rows `Lowering::convert`'s catch-all still names, and the `Core` half of
`as string`.** The file set: `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-runtime/src/helpers.rs`, `crates/mwl-types/src/expr/operators.rs`,
`crates/mwl-stdlib/src/registry.rs`, `tests/conformance/lang/`.

- [ ] **`$m as bytes` and `$m as ?bytes`** — ADR 0007 § 2's tagged-operand-into-`bytes` row, one of
      the three `Lowering::convert`'s catch-all names and the one with a helper already half
      written: `bytes_to_string` is the other direction, and the `?` twin is a `Ty::Bytes` arm
      beside the `Ty::Str` one added this session. Anchors:
      `crates/mwl-ir/src/lower/expr.rs:1265` (the catch-all),
      `crates/mwl-ir/src/lower/expr.rs:1316` (`convert_or_null`),
      `crates/mwl-runtime/src/helpers.rs:985` (`stringify`, the shape to copy).
- [ ] **The `Core`-owned `as string`, refusal half.** `require_stringable_object` exempts every
      `Core` class at its second line, so `echo $uuid` reaches the runtime dispatch, finds no
      compiled method and throws. The standing decision says the registry states which classes have
      a `toString` and a class with none is refused where it is written. Anchors:
      `crates/mwl-types/src/expr/operators.rs:1603` (the `is_core` exemption),
      `crates/mwl-stdlib/src/uuid.rs:150` (a `CoreMethod` row named `toString`),
      `crates/mwl-stdlib/src/registry.rs`.
- [ ] **The `Core`-owned `as string`, rendering half.** A class the registry *does* give a
      `toString` has to reach its native member — `mwl_runtime::stringify` finds nothing in the
      class table, so the row belongs beside `value_to_string`'s carrier one rather than in the
      dispatch. Anchors: `crates/mwl-runtime/src/helpers.rs:985`,
      `crates/mwl-runtime/src/helpers.rs:@value_to_string` (the `Tag::Object` row).

## Backlog

- `array<T> as array<U>` — the O(n) element walk, `Lowering::convert`'s third catch-all target and
  the one that unblocks four playbook bullets; `mwl-ir`'s known gap 4.
- A tagged operand into an object — ADR 0007 § 6's checked way out of `mixed`, which wants a class
  identity `Ty::Object` does not carry; `mwl-ir`'s known gap 4.
- ADR 0010 § 5's integer *into* an enum, in both `as` forms; `mwl-ir`'s known gap 4.
- `mwl-ir` gap 22 — a second file's file-scope statements do not run; `tests/conformance/` cases
  pin a second file by what it declares.
