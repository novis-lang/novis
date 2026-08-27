# Handoff

## State

**M4 — the `as` conversion table is closed**, and closing it found a soundness hole rather
than only a panic. ADR 0007 § 2's table is a *closed* list of rows, so
`mwl_types::expr::operators`' `reject_unconvertible` (`E0708`) refuses every operand/target
pair naming none of them — that function's doc comment is the one home for the rule and for
the row-by-row table behind it.

- **`$foo as Bar` between two unrelated classes read `Bar`'s slot list off a `Foo`.** Both
  erase to one `mwl_ir::ty::Ty::Object`, so the conversion took `Lowering::convert`'s free
  `from == to` row and nothing ran. A class target is therefore judged by
  `types_are_disjoint` rather than by a row (`reject_unrelated_class_conversion`), which
  keeps the three shapes that legitimately name one: a downcast out of plain `object` or an
  interface, a `Core`-owned class deciding for itself (ADR 0024's `as Core\Html\Markup`),
  and the identical type.
- **`null as string` is a lowering row, not a refusal** — the empty string, matching PHP and
  matching what `concat_operand` and `Helper::TaggedToString` already answered for the same
  value. Same decision, same reason, as last session's `concat_operand` arm.
- **`as ?T` is deliberately untouched.** ADR 0066's form interns as `Union([Null, T])` and
  `infer_conversion` skips the new check on the written `?T` sugar; the refusals that form
  still owes are `Lowering::convert_or_null`'s own two panics.
- **`mwl-ir`'s conversion catch-all now has three reachable targets**, all missing
  *lowerings*, all named in the crate docs' gap 20: `array<T> as array<U>`, a tagged operand
  into `bytes`, and a tagged operand into an object.
- Two commits, one per slice; the dispatch-message slice is prose only and owes no case.

## Next group

**The two `as` panics one file over from the table, plus the probe this group did not
reach.** The file set: `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-types/src/expr/operators.rs`, `tests/conformance/lang/`.

- [ ] **`~` and the shift count, one probe deep.** Carried unchanged from the last group.
      `$u << $i` (mixed signedness) is `E0407` through `report_int_uint`, but nothing pins
      that the *count* takes the same row as the operand now that `E0706` sits in front of
      it. One scratch file says whether a case is owed.
      Anchors: `crates/mwl-types/src/expr/operators.rs:@bitwise_result`.
- [ ] **`convert_or_null`'s two panics — ADR 0066 § 3's refusals `mwl_types` does not make.**
      A conversion that *cannot fail* (`$i as ?int`, `$i as ?string`) is that section's
      compile error, and a target with no row at all is the same subtraction this group just
      did one form over. `reject_unconvertible` is the function to reach through: it skips
      `as ?T` today on `is_written_nullable`, so the work is deciding which rows admit the
      non-throwing form and dropping that guard for the rest.
      Anchors: `crates/mwl-ir/src/lower/expr.rs:1292` (`convert_or_null`),
      `crates/mwl-types/src/expr/operators.rs:@reject_unconvertible`.
- [ ] **`lower_to_string_call`'s panic — the object half of `as string`.** A value typed at
      `Stringable` itself and a `Core`-owned class are the two shapes with no recorded
      `toString` target; plain `object`, a shape and a `callable` reach it too, since
      `require_stringable_object` matches `Ty::Class` alone. Each is either a resolution to
      record or an `E`-code beside `E0412`.
      Anchors: `crates/mwl-ir/src/lower/expr.rs:845` (`lower_to_string_call`),
      `crates/mwl-types/src/expr/operators.rs:@require_stringable_object`.

## Backlog

- A tagged operand converted to an object (`$m as Plain` over a `mixed`) — ADR 0007 § 6's
  checked downcast, wanting a helper that takes a `Program::classes` label; `mwl-ir` gap 20.
- `array<T> as array<U>`, the O(n) element walk; `mwl-ir` gap 20 and ADR 0007 § 2 row 6.
- A tagged operand converted to `bytes`, the one scalar target with no runtime-tag row;
  `mwl-ir` gap 20.
- `object` as a declared binding type has no representation arm — `mwl-ir` gap 21.
- A `require`d file's own top-level statements are not run — `mwl-ir` gap 22.
- Stage 8's `every_refusal_is_a_diagnostic_or_decided` guard test does not exist on disk yet;
  `docs/agent/loop-goal.toml` names it and a named check must both exist and pass.
