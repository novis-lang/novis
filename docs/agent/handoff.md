# Handoff

## State

**M4 — language completeness.** **A closure's declared parameter types are checked at the call, and
the one conversion ADR 0007 § 2 admits there is applied rather than refused.** The check itself is
`check_param_tags` (`crates/mwl-runtime/src/closure.rs:325`), reached from `call_closure` and
therefore from both callers — a `Core` member's callback and ADR 0031's `$fn(...)`. Two `.mwlt`
cases under `tests/conformance/lang/` pin it from MWL, each asking the same closure of both callers
so a check that grew a second entry point fails rather than printing plausibly twice.

The widening: an `int` or `uint` arriving at a `float` parameter is converted in place through
`mwl_runtime::helpers::widen_to_float` (`crates/mwl-runtime/src/helpers.rs:750`), which reads the
same `row::int_to_float`/`uint_to_float` a written `as float` does, so the 2^53 boundary cannot
drift between the two spellings. Past it the throw is `ArithmeticError`
(`crates/mwl-runtime/src/closure.rs:392`) — the class ADR 0007 § 4 names, which `helpers`'
`does_not_fit` still cannot reach and says so in its own `# Known gap`. Because the check now
*converts*, `call_closure` builds its `slots` vector first and checks over that copy: the caller
keeps owning the `int` it passed, and neither tag is refcounted, so the retain sweep is unchanged.

The previous handoff's recorded divergence is closed, not carried. Nothing else in this area is
open below the front end.

`verify.py` 6 of 6 green — conformance 619, differential 173.

## Next group

**Pin the same two rules from native code, then settle what a callback's *key* argument carries.**
The file set: `crates/mwl-codegen/tests/closures.rs`, `crates/mwl-runtime/src/closure.rs`,
`crates/mwl-stdlib/src/arr.rs`.

- [ ] **A codegen test that a mismatched argument throws out of a native caller**, beside
      `crates/mwl-codegen/tests/closures.rs:77` — that test already drives a `Core` member into a
      throwing closure and is the harness to copy. Assert the `LogicError` message at
      `crates/mwl-runtime/src/closure.rs:403` and that the partial result leaks nothing, which is
      the half a `.mwlt` case cannot see.
- [ ] **A codegen test for the widening and its bound**, in the same file: an `int` argument
      reaching a `float` parameter as a `float`, and `2^53 + 1` throwing at
      `crates/mwl-runtime/src/closure.rs:392`. Beside it, the unit tests in
      `crates/mwl-codegen/src/ty.rs` that hold `param_tag_nibble` against `tag_of` are where a new
      `Ty` row would have to be noticed.
- [ ] **Decide and pin what a callback's key argument carries.** `Core\Arr::filter` renders every
      key as a `string` before the call (`crates/mwl-stdlib/src/arr.rs:912`), and `map` and the rest
      do the same, so a two-parameter predicate over a *list* that declares `int $k` now throws
      where before the check it read an integer key's payload as an `MwlStr`. Spec § 2's callback
      rule is the home for the answer; whichever way it goes, a `.mwlt` case pins it.

## Backlog

- `Core\Arr::filter`'s `wants_key` reads the arity but not the tags, so a key argument is built for
  a two-parameter closure before anything checks it — `crates/mwl-stdlib/src/arr.rs:892`.
- The remaining M4 language holes, ordered by file set in `docs/agent/loop-goal.md`.
- `python tools/holes.py` is the live worklist; `python tools/loop.py --list` names the `.mwlt`
  cases each stage still owes.
