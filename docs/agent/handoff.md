# Handoff

## State

**Stage 0 items 1, 2, 3, 4, 5, 6 and 9 are done; 7 and 8 are what is left.** ADR 0094's *access* half is
now whole. `E0471` comes from `mwl_types::expr::members::check_member_visibility`
([members.rs:508](../../crates/mwl-types/src/expr/members.rs#L508)), keyed on the accessing class
(`Ctx::current_class`) and never on the receiver's static type; the rule itself is
`signatures::is_visible_from`. A property reaches it through `resolve_property_owned` — `$obj->n`,
`Foo::$n`, and a write through the same `PropertyAccess` span — and a method through
`expr::members::check_method_visibility`, which folds ADR 0043 § 3's private-interface-method refusal in
front of it and reports only that one where both would fire. `$obj->m()`, `C::m()` and `new C(...)` all
take it, so a `private` constructor is PHP's singleton idiom rather than a keyword that means nothing.
The level lives in `MethodSig::visibility` ([signatures.rs:111](../../crates/mwl-types/src/signatures.rs#L111))
and `ClassSignature::property_visibility`, both filled from `signatures::declared_visibility`.

**One declaration shape is deliberately outside it**, recorded in `mwl-types`' `signatures` known gaps and
in `mwl_hir::members`': a promoted constructor parameter, which no table records as a property, so nothing
resolves it to check. `private(set)` is ADR 0094 § 3's write half and is still not modeled.

`python tools/verify.py` is green (1365 tests) and `mwl test tests/` is 429/0; the three `examples/*.mwl`
that declare non-public members still run clean, so nothing in the corpus was reaching a member it should
not have been.

## Next group — Stage 0 item 7, `Comparable`/`Stringable` carry their member signatures

**Shared file set:** `crates/mwl-types/src/iter_lib.rs`, `crates/mwl-hir/src/interfaces.rs`,
`crates/mwl-types/src/lib.rs`'s known gaps and `tests/conformance/lang/`. The rule is
`docs/agent/loop-goal.md` § *Stage 0* item 7; the acceptance name `loop-goal.toml` already waits on is the
`mwl-types` test `a_stringable_parameter_can_call_to_string`.

- [ ] **7a — seed both interfaces' members.** `iter_lib.rs`'s `bodiless`
      ([iter_lib.rs:91](../../crates/mwl-types/src/iter_lib.rs#L91)) is the shape: one `MethodSig` per
      declared member, `has_body: false`, seeded into the `SignatureTable` the same way `Iterable`/
      `Iterator` are. `Stringable` owes `toString(): string`; `Comparable` owes ADR's ordering member —
      read `crates/mwl-hir/src/interfaces.rs:36` for the roster and settle the name against
      `mwl_stdlib`'s `Ordering`/`Order` before writing it. Both take a `visibility: Visibility::Public`
      field now, added this session.
- [ ] **7b — `instanceof Stringable` must record a resolved class.** `mwl-types`' own known gaps
      ([lib.rs:205](../../crates/mwl-types/src/lib.rs#L205)) say it records none today and `mwl-ir` then
      panics; `mwl-ir`'s gap 12 ([lib.rs:218](../../crates/mwl-ir/src/lib.rs#L218)) is the other end, where
      neither `.` nor `as string` covers a `Stringable` operand
      ([ir.rs:489](../../crates/mwl-ir/src/ir.rs#L489)). Close the checker end first and re-read the panic.
- [ ] **7c — the `.mwlt` cases.** A `Stringable` parameter whose `toString()` is called and echoed, and a
      `Comparable` used where `Core\Heap`'s ordering wants one. `tests/conformance/lang/` is picked up with
      no registration.

## Backlog

- Stage 0 item 8: ADR 0061's `autoload` grammar and its name-to-file fixpoint over the require-graph
  worklist `mwl_hir::requires` already walks — `docs/agent/loop-goal.md` § *Stage 0*.
- `mwl-ir` gap 20's enum-case membership row, which also closes ADR 0010 § 5's `int`-into-an-enum
  conversion — `crates/mwl-ir/src/lib.rs`'s known gaps.
- `mixed as int`/`as uint` has no row in `convert` at all, so an `int`-literal set off a `mixed` panics
  before the membership test is reached — same file's gap list.
- ADR 0094 § 3's `private(set)` write half, and a promoted constructor parameter's level — `mwl-types`'
  `signatures` known gaps own both.
- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — plan, `Open now`.
- `mwl-ir` gap 19: `$n + $f` and `$n < $f` still fail in codegen over mismatched representations, though
  `==` no longer does — same file's gap list.
