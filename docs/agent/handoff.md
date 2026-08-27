# Handoff

## State

**M4 — language completeness.** **A closure carries what its parameters require, and the call
checks them.** Two reserved fields sit ahead of the captures now: `FN_ARITY` in slot 0 and
`FN_PARAM_TAGS` in slot 1 (`crates/mwl-ir/src/lower/mod.rs:2680`), the second holding one nibble
per parameter — the `mwl_runtime::Tag` discriminant an argument in that position must carry,
parameter 0 in the low four bits, sixteen parameters to a slot. It is written at the literal
(`crates/mwl-ir/src/lower/expr.rs:3231`) out of `param_tags_word`
(`crates/mwl-ir/src/lower/closure.rs:59`), because the declared types are readable there and
nowhere below this crate.

`check_param_tags` (`crates/mwl-runtime/src/closure.rs:312`) compares one nibble per argument
inside `call_closure`, which is the single path a `Core` member's callback and ADR 0031's
`$fn(...)` both take — putting it in either caller would have left the other holding the hole. A
mismatch is a catchable `LogicError` (`crates/mwl-runtime/src/closure.rs:373`), so
`Core\Arr::map($ints, fn (string $s): string => $s)` throws where it used to read an `int` payload
as an `MwlStr` pointer. That is the priority-1 hole the previous handoff named, closed.

Neither `mwl-ir` nor `mwl-runtime` can name the other, so `param_tag_nibble`
(`crates/mwl-ir/src/lower/mod.rs:2705`) is held against `mwl_codegen::ty::tag_of` by two unit tests
in `crates/mwl-codegen/src/ty.rs` — see the playbook bullet for why that shape rather than a new
dependency edge.

**One divergence, recorded rather than kept quiet:** the comparison is exact, so an `int` argument
to a `float` parameter throws instead of widening, which ADR 0007 § 2 *does* admit at a parameter
position. `check_param_tags`'s own `# Known gap` states it and names the row that closes it; it is
the second slice below.

`verify.py` 6 of 6 green — conformance 617, differential 173.

## Next group

**Pin the check from MWL, then let through the one conversion it refuses.** The file set:
`tests/conformance/lang/`, `crates/mwl-codegen/tests/closures.rs`,
`crates/mwl-runtime/src/closure.rs`.

- [ ] **A `.mwlt` case pinning both sides, through both callers.** A matching call runs and a
      mismatched one is caught, once through a `Core` member (`Core\Arr::filter`) and once through
      `$f(...)`; a `mixed` parameter accepts either. The message to expect is at
      `crates/mwl-runtime/src/closure.rs:373`, and
      `tests/conformance/lang/a-closure-is-called-through-the-variable-holding-it.mwlt` is the
      neighbour whose shape to follow.
- [ ] **An `int` or `uint` argument widens into a `float` parameter instead of throwing** — ADR
      0007 § 2's one implicit conversion, at `crates/mwl-runtime/src/closure.rs:312`, substituting
      the converted value into the slot before the retain loop in `call_closure`
      (`crates/mwl-runtime/src/closure.rs:125`). The 2^53 rule already has one implementation:
      `crates/mwl-runtime/src/helpers.rs:516`. Above it, throw as that row does.
- [ ] **A codegen test that a mismatched argument throws out of a native caller**, beside
      `a_closure_object_carries_its_own_arity_in_slot_zero`
      (`crates/mwl-codegen/tests/closures.rs:27`) — the layout test that says whether slot 1 is
      still where both crates think it is.

## Backlog

- `object` as a declared type has no representation arm — 2 sites, `python tools/holes.py --item 25`.
- ADR 0007 § 4's promotion table still has 11 refusal sites — `python tools/holes.py --item 1`.
- `$x++`/`--$x` (5 sites) and named/spread arguments (3 sites) — items 7 and 16 of the same tool.
- A closure declaring more than 16 parameters is refused at the call and no case reaches it —
  `crates/mwl-runtime/src/closure.rs:312`'s `# Errors`.
- `mwl_runtime::Tag::Array`'s doc comment still says "no representation exists yet"; `Value::array`
  has built one since (`crates/mwl-runtime/src/value.rs:274`).
