# Handoff

## State

**M4 — language completeness.** Two more `mwl-ir` catch-alls have no reachable target, and
both were closed by widening a *checker* refusal rather than by adding a lowering row.

- **The `decimal` operator table** (`lower_decimal_binary`). The item's own survivor list was
  wrong and the roster is why it is subtracted by hand: `Pow`, `Concat`, `And`, `Or` and
  `Coalesce` are five of ten, and the five it missed — `BitAnd`, `BitOr`, `BitXor`, `Shl`,
  `Shr` — were the ones that actually reached the panic. `E0706` now refuses every operand of
  `& | ^ ~ << >>` outside `int`/`uint`, which is ADR 0007 § 4's whole row. That refusal is
  worth more than the panic it removes: `bitwise_result`'s silent `_ => mixed` arm meant
  `1.5 & 1.5` **evaluated to `1.5`** (a bit-and over the `f64`'s bits) where PHP answers `1`,
  and `~1.5` reached `mwl-codegen`'s "not lowered yet". `~` is in the same code by design —
  a `float`/`decimal` operand is a number with no bit pattern, so it takes the bitwise
  sentence, and every *non*-numeric operand of `~` keeps `E0705`.
- **`concat_operand`'s representation catch-all.** `Ty`'s fifteen are nine rows, four
  refusals and two representations no source expression has (`ClassDesc` is only a static
  call's receiver slot, `Ref` only a staged `&$x`). `E0707` refuses `bytes`, `array<T>`, an
  enum case and a `void` call at the four implicit sites, which are one check —
  `require_stringable`, now the whole of ADR 0007 § 2's "anything → `string`" row, with the
  object half split out as `require_stringable_object` so the explicit `as string` keeps ADR
  0009 § 3's `bytes` conversion.
- **`null` renders as the empty string** rather than being refused: the `?string` holding one
  already printed nothing through `Helper::TaggedToString`, and PHP agrees, so refusing the
  static case would diverge from both. Decided under the goal's standing "decide and record";
  its home is `concat_operand`'s own arm comment plus `E_NO_STRING_FORM`'s doc.
- **One code commit for two slices again, deliberately** — both edit the same three files
  (`mwl-diagnostics/src/lib.rs`, `mwl-types/src/expr/operators.rs`,
  `mwl-ir/src/lower/expr.rs`) and a commit stages whole files.

## Next group

**The two conversion catch-alls, one file over from the last three.** `as` is where the
remaining panics live, and the dispatch message is the small one beside them. The file set:
`crates/mwl-ir/src/lower/expr.rs`, `tests/conformance/lang/`.

- [ ] **`expr.rs:1240` — the `as` conversion table's catch-all.** Confirmed live, not
      hypothetical: `$xs as string` over an `array<int>` panics there with `got
      Array as Str`. Subtract the operand × target grid against ADR 0007 § 2's table, ADR
      0009 § 3 and ADR 0010 § 5 — the message itself names `array<T> as array<U>` and a
      `Ty::Tagged` operand into `bytes` as the two known gaps, so what is left to decide is
      every *other* survivor: `Array as Str` (`Core\Json::encode` is the spelling that
      exists), `Enum as Str`, `Void as` anything, `Null as Str`. Some want a lowering row and
      some an `E07xx`; `E0707`'s wording is the precedent for the refused half.
      Anchors: `crates/mwl-ir/src/lower/expr.rs:1240`,
      `crates/mwl-types/src/expr/operators.rs:75` (the `as` site that calls
      `require_stringable_object`).
- [ ] **`expr.rs:384` — the `lower_expr` dispatch's message.** The *arm* was proven
      unreachable two sessions ago; what is left is the message, which still reads as a
      to-do list rather than as an assertion. Rewrite it in the shape the other four now
      share — roster subtracted in the comment, `"mwl-ir: unreachable — …; see this arm's
      own comment"` below it. No behaviour change, so no new case.
      Anchors: `crates/mwl-ir/src/lower/expr.rs:384`.
- [ ] **`~` and the shift count, one probe deep.** `$u << $i` (mixed signedness) is
      `E0407` through `report_int_uint`, but nothing pins that the *count* takes the same
      row as the operand now that `E0706` sits in front of it. One scratch file says whether
      a case is owed.
      Anchors: `crates/mwl-types/src/expr/operators.rs:@bitwise_result`.

## Backlog

- `Ty::Iterable` and `Ty::Never` have no `erase_checked_ty` row, so they panic in
  `mwl-ir/src/lower/mod.rs:2650` rather than at a use site — `docs/plan/m4.md`.
- A closure or a shape in an `echo` reaches `Helper::TaggedToString` and throws at run time
  rather than being refused where it is written; ADR 0028 § 1 would allow either.
- `spawn script` (`E0703`) and `require` for its value (`E0704`) stay open at M5 and
  `mwl-ir`'s known gap 22.
- `python tools/holes.py` is the live worklist for what is left below `mwl-ir`.
