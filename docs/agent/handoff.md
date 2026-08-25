# Handoff

## State

**Stage 0 items 1, 2, 3, 4 and 9 are done; item 5 is next.** ADR 0090 is built end to end. The last row —
§ 5's numeric one — closed this session: `shallow_identical`'s three separate numeric arms are gone, and
`value_identical` delegates all four numeric representations to `numeric_identical`, which grew the
`decimal` rows over `Decimal::compare` and `Decimal::compare_f64`. So the row has **one** answer however
it is reached, and `Core\Arr` takes it too — `Arr::contains([1.0], 1)` is now `true`, a deliberate
divergence from PHP's `===` that the strict-identity standing decision pre-authorized.

`value_hash` had to move with it, and the shape is worth knowing before touching it: a numeric hashes as
**the `f64` it coincides with**, when there is one, and as its own exact reduced decimal otherwise. That
is the only key both of the domain's readings of a float agree on — an integer is compared to a float at
the float's exact binary value, a `decimal` at the decimal it prints as, so past 2^53 one float is
identical to two values that are not identical to each other. `hash_numeric`'s own doc comment owns that
reasoning, including why `coincident_float` refuses a value that merely *rounds* to a float: merging the
1023 integers beside a large float would hand an attacker a fillable bucket. `crates/mwl-stdlib/src/arr.rs`
needed no code change — its `Identity` wrapper already delegates both halves.

`python tools/verify.py` is green (1331 tests). No valgrind run: every representation in the new arms is a
scalar, so the slice adds no refcount edge.

## Next group — ADR 0047 § 4's literal and enum-case type atoms (Stage 0 item 5)

**Shared file set:** `crates/mwl-types/src/ty.rs` and `src/lower.rs` for all three slices, then
`src/expr/assign.rs` and `crates/mwl-types/tests/literal_types.rs`. `loop-goal.toml`'s `mwl-types` block
pins the finished name: `a_literal_type_atom_is_checked`.

- [ ] **Item 5a — the three atoms intern as types instead of being refused.** `lower_type_at_depth` at
      [lower.rs:145](../../crates/mwl-types/src/lower.rs#L145) sends `TypeAtom::StringLiteral`,
      `IntLiteral` and `Member(..)` to `reject_unchecked_literal_type`
      ([lower.rs:156](../../crates/mwl-types/src/lower.rs#L156)); they get real `Ty` variants in the enum
      at [ty.rs:28](../../crates/mwl-types/src/ty.rs#L28) instead. § 2 folds a class constant to its own
      literal type and § 3 keeps an enum case a *narrowed subtype* of its enum rather than its backing
      value — those are two different variants, not one. § 5 promises **zero** additional runtime
      representation, so nothing in `mwl-ir` may learn a new `Ty`; each erases to its base on the way out.
- [ ] **Item 5b — § 4's assignability and conversion table.** A literal type is assignable to its base and
      the base is not assignable to it; a union of them is ADR 0047's closed set, and narrowing on an `==`
      against one is ADR 0007 § 6's job in `expr/assign.rs` beside `is_assignable`. Delete
      `reject_unchecked_literal_type` and retire `E0457` (`E_LITERAL_TYPE_UNCHECKED`,
      [mwl-diagnostics/src/lib.rs:617](../../crates/mwl-diagnostics/src/lib.rs#L617)) — retired, never
      reused, per conventions.md.
- [ ] **Item 5c — § 6's diagnostics and the test rename.** `crates/mwl-types/tests/literal_types.rs`
      currently asserts the *refusal*; its cases become § 4's table, with the one
      `loop-goal.toml` names. A `.mwlt` case under `tests/conformance/lang/` pins the runtime half —
      § 5 says there is none, so the case is that a literal-typed binding behaves exactly as its base.

## Backlog

- Item 6 — `private`/`protected` are enforced at an access site (loop-goal.md § *Stage 0*).
- Item 7 — `Comparable`/`Stringable` carry their member signatures; `instanceof Stringable` panics
  `mwl-ir` today (loop-goal.md § *Stage 0*).
- Item 8 — ADR 0061's `autoload` grammar and its name-to-file fixpoint (loop-goal.md § *Stage 0*).
- `+` and `<` over two numeric representations still fail in codegen — ADR 0007 § 4's promotion table,
  `mwl-ir`'s gap 19. `==` is out of that hole; the arithmetic operators are not.
- ADR 0088's registry-wide qualifier classification lands with M4S (implementation-plan.md § *Open now*).
- Stage 3 resumes at `examples/collect.mwl`: `Core\Path` first, then `Encoding`/`Hash`/`Uuid`
  (implementation-plan.md § *Blocking*).
