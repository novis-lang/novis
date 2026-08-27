# Handoff

## State

**M4 — a digit run beside a `uint` is placed at `uint`**, which is ADR 0007 § 2's "a numeric
literal is untyped until placed" applied to the one placement a binary operator offers: its
other operand. `mwl_types::expr::literals`' `uint_operand_expectation` is the one home for the
rule and for why it is value-preserving on every row of § 4's table.

- **Before this, `uint` could not meet a literal at all.** `$u + 1`, `$u & 3` and `$u << 1`
  were each `E0407`, so a `uint` operand had to be paired with a `uint`-declared local — which
  is why the bitwise sweep beside the new case writes `uint $uOne = 1;`. A compound assignment
  never had the problem: `check_compound_assign` already hands its value the target's type as
  a hint, and this is that hint for the spelling that was missing it.
- **`mwl-ir` makes the same placement**, and had to: `lower_binary` already passed `Some(lty)`
  to its right operand, so only the *left*-hand digit run fell through to `ConstInt` and
  panicked on a value above `i64::MAX`. Both crates now lower the literal side second; a
  literal is a constant with no effects, so nothing is observably reordered.
- **The shift count stays an operand, not a width.** `$u << $i` is `E0407` in both directions,
  and that refusal is load-bearing: `mwl-codegen`'s `emit_shift` reads one signedness for both
  the negative-count guard and the arithmetic-versus-logical choice, which is sound only
  because a `uint` operand cannot have a signed count. This was the item's probe, and the
  answer was that a case was owed.
- **The sibling gap is an array literal's elements**, which still keep `int` whatever the
  declared element type says — the playbook bullet under *Writing MWL itself* has it, and it
  is now the last position ADR 0054 § 2's rule is not applied at.

## Next group

**The two `as` panics in `lower/expr.rs`, plus the placement position still missing.** The file
set: `crates/mwl-ir/src/lower/expr.rs`, `crates/mwl-types/src/expr/literals.rs`,
`tests/conformance/lang/`.

- [ ] **`convert_or_null`'s two panics — ADR 0066 § 3's refusals `mwl_types` does not make.**
      `as ?T` interns as `Union([Null, T])` and `infer_conversion` skips `reject_unconvertible`
      on the written `?T` sugar, so the refusals land nowhere. Anchors:
      `crates/mwl-ir/src/lower/expr.rs:1294`, `crates/mwl-types/src/expr/operators.rs:107`.
- [ ] **`lower_to_string_call`'s panic — the object half of `as string`.** A value typed at
      `Ty::Object` with no `Stringable` proof reaches it. Anchors:
      `crates/mwl-ir/src/lower/expr.rs:845`, `crates/mwl-ir/src/lower/expr.rs:1001`.
- [ ] **An array literal's elements are the last position ADR 0054 § 2's placement is not
      applied at.** `array<uint> $u = [7, 8];` compiles and its elements carry the `int` tag.
      Anchors: `crates/mwl-types/src/expr/literals.rs:718`,
      `crates/mwl-types/src/expr/mod.rs:210`.

## Backlog

- `array<T> as array<U>` has no lowering — `mwl-ir` crate docs, gap 20.
- A tagged operand into `bytes`, and into an object — same gap 20.
- `mwl-ir` gap 1: `Class::method(...)` first-class callable has no resolved target.
- `mwl-ir` gap 22: `require` used for its value (`E0704`).
