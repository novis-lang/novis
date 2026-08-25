# Handoff

## State

**Stage 0 items 1, 2, 3, 4 and 9 are done; item 5 is in progress — 5a, 5b and 5c landed, 5d is next.**
ADR 0047 is whole in the checker. § 4's checked `as` now places a *string* literal operand at its target
the way ADR 0054 § 2 already placed a numeric one
([operators.rs:68](../../crates/mwl-types/src/expr/operators.rs#L68)), so `"a" as "a"|"b"` is statically
satisfied rather than converting a plain `string`, and `reject_impossible_literal_conversion`
([operators.rs:686](../../crates/mwl-types/src/expr/operators.rs#L686)) refuses the conversion an operand's
own value disproves — **E0469** for a closed set of literals, **E0470** for one of enum cases, each naming
the accepted set generated from the target type. Both halves have to be closed for that proof: a target
with one wider atom (`string`, or the `null` an `as ?T` adds) and an operand that names more than one value
are § 4's ordinary checked row and still compile. ADR 0047 § 6 now carries the two codes.

`python tools/verify.py` is green (1348 tests). No valgrind run: nothing here reaches the heap.

**Not yet true, and expected:** nothing lowers § 5's membership test, so **no literal-typed conversion runs
at all** — `$raw as "a"|"b"` panics `Str as Tagged` at
[mwl-ir/src/lower/expr.rs:656](../../crates/mwl-ir/src/lower/expr.rs#L656), because a union of literal
atoms erases to `Ty::Tagged` rather than to the one base its members share. That is 5d's first half, and it
is why the new conformance case pins the two *compile* errors only.

## Next group — ADR 0047 § 5's runtime half (Stage 0 item 5d)

**Shared file set:** `crates/mwl-ir/src/lower/mod.rs`, `crates/mwl-ir/src/lower/expr.rs` and
`tests/conformance/lang/`.

- [ ] **5d-1 — a closed set of literals erases to the base its members share.** `lower_checked_ty`
      ([lower/mod.rs:1849](../../crates/mwl-ir/src/lower/mod.rs#L1849)) already erases each atom; its
      `Union` arm does not fold `"a"|"b"` back to `Ty::Str`, so the target of every literal conversion
      arrives as `Ty::Tagged` and [expr.rs:656](../../crates/mwl-ir/src/lower/expr.rs#L656) panics. ADR
      0047 § 5 is unambiguous that there is no second representation — a union whose members all erase to
      one `Ty` is that `Ty`.
- [ ] **5d-2 — the membership test itself.** `lower_conversion`
      ([expr.rs:2838](../../crates/mwl-ir/src/lower/expr.rs#L2838)) reads its target off the *annotation*
      before erasure already (`nullable_target`, the same trick ADR 0066 needed), which is where the
      literal **set** has to be read too: after `lower_decl_type` the values are gone. It throws on a
      value the set does not contain, beside the `uint`/enum conversion § 4 calls it a further-restricted
      version of — and that enum row is the *same* missing check, so build one and use it twice.
- [ ] **5d-3 — a `.mwlt` case under `tests/conformance/lang/`** pinning that the conversion throws off
      the happy path and that a literal-typed binding costs nothing extra when it does not.
      `a-conversion-the-operand-disproves-is-a-compile-error.mwlt` is the compile-error sibling; this one
      is the running half, and needs 5d-1 and 5d-2 before it can pass.

## Backlog

- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — plan, `Open now`.
- `private`/`protected` enforcement (Stage 0 item 6) — `docs/agent/loop-goal.md` § *Stage 0*.
- `Comparable`/`Stringable` member signatures (item 7) — same file.
- ADR 0061's `autoload` grammar and name-to-file fixpoint (item 8) — same file.
- `equality_domain` puts a literal type in its base's ADR 0090 domain, so `$mode == "z"` compares two
  strings; refusing non-overlapping literal *sets* would be a new row in ADR 0090 § 2.
- `Core` breadth resumes at `examples/collect.mwl` — plan, `Open now`.

`orient.py` printed everything this session needed for the checker half. One gap for the manifest, and 5d
will pay it in full: `[context] modules` selects `mwl-ir/src/lower/*.rs`'s module lines but nothing from
`mwl-runtime`, and the membership test is likely to want a runtime helper beside `mwl_runtime::identity`.
