# Handoff

## State

**M4 — language completeness.** Two of the array-literal element list's three slices
landed; the third (the lowering) is the whole of the next group.

`&value` as an array-literal element is **E0483** and no longer a panic. The standing
decision in `docs/agent/loop-goal.md` is what it names: ADR 0031 § 2 removed
by-reference capture and ADR 0023 fixes an element as a copy taken where the literal is
evaluated, so an aliasing element has no owner in either — a shape the language does not
have, not a lowering `mwl-ir` has not learned. It is reported per `ArrayItem` in
`check_array_literal` (`crates/mwl-types/src/expr/literals.rs:596`), so every element
list is walked at every depth and a nested `&` is refused where it is written;
`lower_array_literal`'s assert (`crates/mwl-ir/src/lower/expr.rs:3643`) names only
`...spread` now, and `mwl-ir`'s gap 8 says so.

`[...$a]` type-checks, in the same loop and by the same move the written-out elements
use: `check_spread_element` (`crates/mwl-types/src/expr/literals.rs:668`) gives the
subject `array<T>` for the literal's own `T` as its **expectation**, so a mismatch is an
ordinary `E0401` naming both array types, and element covariance falls out of it rather
than being a second rule. **E0484** is only what a position naming no `array<T>` at all
is left with — a `mixed` binding or parameter — and the `env.diags.len()` guard is what
keeps `[...$s]` from being two diagnostics. That function's doc comment owns the rest,
including the decision the next slice needs: **nothing is recorded on the `ExprInfo`
side for a spread**, because the lowering reads `ArrayItem::spread` off the AST and its
own `lower_expr` answers the subject's `Ty::Array`.

`verify.py` 6 of 6 green — conformance **583**, differential 162. `python
tools/holes.py` is unchanged at **35 sites, 9 items**: item 17's site is the single
assert line both spellings shared.

## Next group

**The spread family**, and the first two share `crates/mwl-ir/src/lower/expr.rs`'s
`lower_array_literal` (line 3643) with `crates/mwl-ir/src/lower/mod.rs`'s array-literal
fixture block (around line 4020, whose `should_panic` fixture is the one to delete).
Item 17, then item 16, in `python tools/holes.py`.

- [ ] **`[...$a]` lowers** — `crates/mwl-ir/src/lower/expr.rs:3643`, appending the
      subject's entries into the literal's array, in both shapes the arm already has
      (all-positional `ArrayNew`, and `ArrayNew` + `ArraySet`* once any key is
      explicit). What arrives is well-typed now, so the subject's `lower_expr` answers a
      `Ty::Array` and nothing has to be re-checked. Refcounts: each entry copied out of
      the subject is a fresh reference the new array owns, which is the same rule the
      written-out elements two lines up already follow through `aliasing_read`.
- [ ] **The key semantics, decided and pinned** — **one call to take, not to `BLOCKED`**:
      PHP renumbers integer keys and preserves string ones, while ADR 0007 § 5 makes
      *every* MWL key a string, so "renumber the integer-looking ones" and "preserve
      every key" are both defensible. Record the answer in ADR 0007 § 5 and pin it with
      a `tests/differential/` case, PHP being the oracle for exactly this.
- [ ] **Item 16's checker half: a named or spread *call* argument type-checks** —
      `crates/mwl-types/src/expr/args.rs:38` (`check_args_typed`) and `:117`
      (`check_arg`). The spread half is `check_spread_element`
      (`crates/mwl-types/src/expr/literals.rs:668`) one position along: a variadic
      parameter's element type is the expectation, and a subject that is not an
      `array<T>` takes the same refusal. `loop-goal.md` § *Standing decisions* fixes the
      order — checker before lowering — so `crates/mwl-ir/src/lower/call.rs:75` stays a
      panic this group.

## Backlog

- `every_refusal_is_a_diagnostic_or_decided` **does not exist in `crates/`**, though
  `loop-goal.toml`'s Stage 8, `loop-goal.md` and the plan all discuss its allowlist as
  though it were on disk (`docs/agent/loop-goal.toml`).
- Item 1's and item 25's 13 catch-all sites are misattributed in `tools/holes.py`; the
  worklist would read truer if they were split off (`docs/implementation-plan.md`).
- `lower_decl_type`/`lower_checked_ty`'s uncovered *declared* types — `decimal`,
  `never`, `iterable`, `self`/`static`/`parent`, a shape, an intersection — belong to
  no item yet (`docs/implementation-plan.md` § *Open now*).
- `Core\Json::decodeAs<T>`'s wider codec-reachable set (`mwl_stdlib::json` gaps).
- ADR 0088's qualifier classification (`mwl_stdlib::hash`'s module doc).
