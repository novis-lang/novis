# Handoff

## State

**M4 — ADR 0066 § 3's table is closed at both ends**, which is the `as ?T` half of the
subtraction ADR 0007 § 2's table already finished. `mwl_types::expr::operators`'
`reject_unavailable_nullable_conversion` is the one home for the rule, and
`nullable_conversion_is_total` for which rows cannot fail.

- **The sugar took one refusal of its own and shares the other.** A row that cannot fail is
  `E0709` naming `as T` — the identity, ADR 0007 § 2's total "anything → `string`", ADR 0035's
  `as bool`, ADR 0010 § 5's enum → its backing type, the widening row, and an enum case into
  its own enum. A pair naming no row is `E0708`, § 2's closure asked of the `T` inside the
  sugar; `infer_conversion` used to skip it for every written `?T`, because the target interns
  as one `ConvKind::Wide` union that says nothing to the table.
- **`nullable_conversion_is_total` and `mwl_ir::lower::expr`'s `conversion_can_fail` have to
  agree** — that one picks the `?` helper for a row this one leaves standing. Two rows are the
  checker's alone, a representation being unable to see them: a value that already *is* one of
  the target's, and § 3 row 2's literal/enum-case/whole-enum targets, whose membership test is
  fallible however the base reads.
- **`convert_or_null`'s two panics now name lowering gaps only**, and `mwl-ir`'s known gap 4
  is their home: a `?T` target that produces text or a container has no helper. `$m as ?string`
  is the reachable one and wants a `Helper::TaggedToString` twin answering `null`.
- **Only one slice of the group was taken.** The second turned out to be an investigation
  rather than a follow-on — see below; the third is untouched and moved to the backlog.

## Next group

**The object half of `as string`, and the `?` twin that wants the same helper.** The file set:
`crates/mwl-ir/src/lower/expr.rs`, `crates/mwl-types/src/expr/operators.rs`,
`crates/mwl-runtime/src/helpers.rs`, `tests/conformance/lang/`.

- [ ] **`lower_to_string_call`'s panic, the `Stringable`-typed receiver.** `Stringable $s = new
      Tag(); $s as string;` panics, and the cause is *not* where the message points: this
      session read out that `implements_interface` answers `true` for the self pair
      (`crates/mwl-hir/src/hierarchy.rs:380`) and that `iter_lib` seeds `Stringable::toString`
      into the signature table, so `require_stringable_object` passes its gate and then either
      `resolve_method` misses or `record_to_string` keys a span the lowering does not look up.
      One `dbg!` at `crates/mwl-types/src/expr/operators.rs:1629` settles which. Anchors:
      `crates/mwl-ir/src/lower/expr.rs:1133`, `crates/mwl-ir/src/lower/expr.rs:845`.
- [ ] **The same panic's erased half.** `require_stringable_object` returns at its first line
      for every operand that is not a `Ty::Class` — plain `object`, a shape, a `callable` — so
      `object $o = new Tag(); $o as string;` reaches the panic with nothing recorded and
      nothing refused. ADR 0007 § 2 makes `Stringable` what an object owes, and a plain
      `object` proves none, so the answer is a diagnostic where it is written. Anchors:
      `crates/mwl-types/src/expr/operators.rs:1599`.
- [ ] **The same panic's `Core` half**, which the standing decision already answers: a `Core`
      class is stringifiable exactly where the spec gives it a `toString`, `mwl_stdlib::registry`
      states it, and one with none is an ordinary `E`-code at the `echo`. `qname.is_core()` is
      the early return that skips both today. Anchors:
      `crates/mwl-types/src/expr/operators.rs:1603`.
- [ ] **`Helper::ToStringOrNull` — the `?string` twin**, which closes the reachable half of
      `convert_or_null`'s remaining panic. Five touch points, all named by `ToIntOrNull`:
      `crates/mwl-ir/src/ir.rs:1528`, `crates/mwl-ir/src/print.rs:483`,
      `crates/mwl-codegen/src/emit.rs:3254`, `crates/mwl-runtime/src/helpers.rs:796`, and
      `crates/mwl-ir/src/lower/expr.rs:1308`. `value_to_string` is the row set it answers over.

## Backlog

- **An array literal's elements are the last position ADR 0054 § 2's placement is not applied
  at** — `array<uint> $u = [7, 8];` compiles with `int`-tagged elements.
  `crates/mwl-types/src/expr/literals.rs:718`, `crates/mwl-types/src/expr/mod.rs:210`.
- **A `?T` whose `T` is itself a union** (`$m as ?(int|string)`) is judged by neither half: the
  target has more than one non-`null` member, so the checker leaves it alone and it reaches
  `convert_or_null`'s `from == to` panic. `crates/mwl-ir/src/lower/expr.rs:1302`.
- `$m as ?bytes`, `$m as ?array<T>` and `$m as ?object` are `Lowering::convert`'s own missing
  rows in the null-answering spelling, and close with them — `mwl-ir`'s known gap 4.
- `python tools/holes.py` attributes no item to `convert_or_null`'s panics, so its detector
  misses a refusal inside a `match` arm; 2 sites in `mwl-codegen/src/ty.rs` are unattributed too.
- ADR 0007 § 2's `array<T> as array<U>` row still does not lower, in either spelling.
- ADR 0010 § 5's integer *into* an enum needs the declaration's case set carried to the check.
