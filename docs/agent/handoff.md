# Handoff

## State

**M4 — language completeness.** The array-literal/`??` group landed whole: an array
literal placed against a `?array<T>` expectation is an `array<T>` in a binding, a
`return` and an argument, and `??` guards **every** level of a subscript chain under it
rather than only its immediate operand.

Both halves are the same one move, `TypeInterner::without_null` before the `Ty::Array` is
read off a type. In `check_array_literal` (`crates/mwl-types/src/expr/literals.rs:596`)
it is the *expectation* whose `null` is stripped; in the `ExprKind::Index` arm
(`crates/mwl-types/src/expr/mod.rs:346`) it is the *base*, and there only when the read
is coalesce-guarded, because under a `??` a `null` base has an answer rather than being
the untested nullable `E0482` refuses. The marking walks the whole `Index` chain
(`crates/mwl-types/src/expr/mod.rs:204`), and `Env::coalesce_guarded`'s doc comment
(`crates/mwl-types/src/lib.rs:369`) owns why. The runtime half is three lines:
`mwl_array_optional_get` (`crates/mwl-runtime/src/helpers.rs:287`) answers `null` for a
`null` array, so a guarded read whose base is another guarded read needs no branch in
`mwl-ir` at all. Nothing in `mwl-ir` or `mwl-codegen` changed but a doc comment — the
helper ABI already passes a `Ty::Tagged` operand, and `aliasing_read` already answered
correctly for a chain rooted at a variable.

`verify.py` 6 of 6 green — conformance **581**, differential 162. `tools/leak-check.sh`
green over both new fixtures. `holes.py` unchanged at **35 sites, 9 items**; neither of
these was a panic.

**Item 1 is behaviourally closed** and its 11 sites are catch-alls, like item 25's: every
promotion row in ADR 0007 § 4 runs, checked by hand this session. The playbook bullet
claiming `$n + $f` and `$n < $f` still fail at `emit_binop` is corrected in place.

## Next group

**All three are the array-literal element list**, and they share
`crates/mwl-types/src/expr/literals.rs`'s `check_array_literal` (line 596) and
`mwl-ir`'s `ArrayLiteral` arm (`crates/mwl-ir/src/lower/expr.rs:3651`, whose panic names
both spellings at once). Item 17 in `python tools/holes.py`.

- [ ] **`&value` as an array-literal element is a diagnostic, not a panic** — the
      standing decision in `docs/agent/loop-goal.md` settles it: ADR 0031 § 2 removed
      by-reference capture and ADR 0023 fixes what a copy means, so an aliasing array
      element has no owner in either. New code in the types band (**E0483** is next
      free), declared in `crates/mwl-diagnostics/src/lib.rs`, reported from
      `check_array_literal` over an `ArrayItem` carrying the `&`, with help naming that
      decision. This is half of item 17's site.
- [ ] **`[...$a]` type-checks** — the spread subject's element type has to satisfy the
      literal's, checked in the same loop in `check_array_literal`
      (`crates/mwl-types/src/expr/literals.rs:608`); a subject that is not an
      `array<T>` is a diagnostic beside the `&value` one. Nothing records a spread on
      the `ExprInfo` side yet, so decide here whether the lowering reads the AST again
      or gets a table entry.
- [ ] **`[...$a]` lowers** — `crates/mwl-ir/src/lower/expr.rs:3651`, appending the
      subject's entries into the literal's array. **One semantics call to take and
      record** (do not `BLOCKED` it): PHP renumbers integer keys and preserves string
      ones, while ADR 0007 § 5 makes *every* MWL key a string, so "renumber the
      integer-looking ones" and "preserve every key" are both defensible. Record the
      answer in ADR 0007 § 5 and pin it with a `tests/differential/` case, since PHP is
      the oracle for exactly this.

## Backlog

- Item 16: a named/spread *call* argument, `crates/mwl-ir/src/lower/call.rs:71`,`:77` —
  the checker half first, per `loop-goal.md` § *Standing decisions*.
- Item 1's and item 25's 13 catch-all sites are misattributed in `tools/holes.py`; the
  worklist would read truer if they were split off (`docs/implementation-plan.md`).
- `lower_decl_type`/`lower_checked_ty`'s uncovered *declared* types — `decimal`,
  `never`, `iterable`, `self`/`static`/`parent`, a shape, an intersection — belong to
  no item yet (`docs/implementation-plan.md` § *Open now*).
- `Core\Json::decodeAs<T>`'s wider codec-reachable set (`mwl_stdlib::json` gaps).
- ADR 0088's qualifier classification (`mwl_stdlib::hash`'s module doc).
- Conformance is 581 of the goal's 750; `python tools/gaps.py` still ranks the thin
  `Core` classes for a session whose group is blocked.
