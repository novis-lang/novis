# Handoff

## State

**Stage 0 items 1, 2, 3, 9 and the § 3 half of item 4 are done.** ADR 0090 § 3's table now runs whole,
and each row enters `mwl_runtime::identity` by the cheapest door its operands' static types justify: a
string pair calls `mwl_str_eq`, an array pair the new `mwl_array_eq`, an object pair an inline `icmp` that
calls nothing, and a `mixed`/union operand — § 5, the one row whose answer is a runtime tag — the new
`Helper::Identical` over `mwl_value_identical` on ADR 0002's helper convention. `identity.rs`'s module doc
owns that four-door choice and why only § 5 pays the convention. § 5's "a mismatched pairing is `false`,
never a throw" is `shallow_identical`'s fall-through arm rather than a second rule.

`RuntimeSig::StrEq` is now `RuntimeSig::PtrEq` (`Signatures::ptr_eq`) because `mwl_str_eq` and
`mwl_array_eq` are one shape. `mwl-ir`'s **gap 19** is rewritten: § 3 is built, § 2's numeric domain is
what is left.

`python tools/verify.py` is green (1330 tests), and `tools/leak-check.sh` under valgrind is clean over two
scratch fixtures exercising the new release edges. One conformance case was added,
`tests/conformance/lang/equality-over-arrays-objects-and-mixed.mwlt`.

**Still open, and it is the next group:** ADR 0090 § 2 makes `int`/`uint`/`float`/`decimal` one domain, so
`mwl-types` accepts `$n == $f` — but `mwl-codegen` refuses it at
[emit.rs:861](../../crates/mwl-codegen/src/emit.rs#L861) (*"a binary operator over mismatched
representations"*). Those rows are pinned in `crates/mwl-types/tests/equality.rs` and deliberately left out
of the conformance case, which says so in a comment.

**`orient.py` did not print `mwl-codegen` at all**, and `[context] modules` in `loop-goal.toml` names only
`identity.rs`/`value.rs` under `mwl-runtime` and only `lower/*` under `mwl-ir`. This session also needed
`crates/mwl-codegen/src/emit.rs` and `src/lib.rs`, `crates/mwl-runtime/src/helpers.rs`, `array.rs`,
`string.rs` and `lib.rs`, and `crates/mwl-ir/src/ir.rs` and `print.rs` — the next group needs the first two
again, so `[context] modules` wants a `mwl-codegen` pattern above all.

## Next group — ADR 0090 § 2 at run time (Stage 0 item 4, the half left)

**Shared file set:** `crates/mwl-codegen/src/emit.rs`, `crates/mwl-ir/src/lower/expr.rs`, then the two test
files. Two slices, not three: [2] cannot be written until [1] lowers.

- [ ] **Item 4b — one side widened before a cross-representation numeric compare** (ADR 0090 § 2's numeric
      row). `emit_binop` at [emit.rs:849](../../crates/mwl-codegen/src/emit.rs#L849) refuses `ty != rty` at
      [emit.rs:861](../../crates/mwl-codegen/src/emit.rs#L861); the decision to make is *where* the widening
      goes — a conversion emitted in `lower_binary` at
      [expr.rs:1946](../../crates/mwl-ir/src/lower/expr.rs#L1946), which keeps codegen's one-representation
      invariant, or an inline `sextend`/`fcvt` in `emit_binop`, which does not. Prefer the first: the
      `decimal` and `Tagged` arms directly above it already settle their pairing in lowering, so this is
      the third instance of a rule, not a new one. `int`/`uint` widen through `i128`-equivalent care the
      way `mwl_runtime::identity::integer` does — `-1 as int` and `u64::MAX as uint` share a bit pattern
      and are two values.
- [ ] **Item 4c — restore the dropped conformance rows.** `the_numeric_types_are_one_domain` at
      [equality.rs:117](../../crates/mwl-types/tests/equality.rs#L117) pins the rows that only type-check;
      once [1] lowers them, move them into
      `tests/conformance/lang/equality-across-overlapping-types-still-compiles.mwlt` and delete the comment
      at its head that says why they are absent.

## Backlog

- ADR 0047 § 4's literal and enum-case type atoms are refused by name — Stage 0 item 5, `mwl-types`.
- `private`/`protected` are enforced nowhere at an access site — Stage 0 item 6, `mwl-types`' gap list.
- `Comparable`/`Stringable` carry no member signatures, so `instanceof Stringable` panics `mwl-ir` — item 7.
- ADR 0061's `autoload` grammar and its name-to-file fixpoint — Stage 0 item 8, `mwl-hir`.
- An abandoned generator never runs the `finally` it is suspended inside — `mwl-ir`'s gap 18.
- `Core\Path` is the cheapest slice of `examples/collect.mwl` — `docs/implementation-plan.md` § *Open now*.
