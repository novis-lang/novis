# Handoff

## State

**Goal `bigint` is finished.** The class, its 24 members, seven conformance cases,
`examples/bigint.nvs` and the three guard tests in `crates/nvs-stdlib/src/bigint.rs`'s own `mod tests`
are on disk, and every stage-4 and stage-5 check in `docs/agent/goals/69-bigint.toml` passes by hand
today.

**The natural order over objects is pinned where it is read.** `Core\Arr::min`/`max` and
`Core\Math::min`/`max`/`clamp` agree with `compareTo` over a `Core\BigInt` table — counted over every
ordered pair rather than read off a line — and all seven members that reach
`crate::ordering::compare_values` refuse a class that declares none, each naming itself in front of the
one shared message. `rule:classes/comparable` is the rule; the sort spelling was already pinned.

**The live `docs/agent/loop-goal.*` is goal `test-doubles`, and its stage-6 gate is green by hand.**
The `Core\Test` key is struck, both rules are `shipped`, the rulebook renders clean, neither
`owners.py --closes` nor `playbook.py --closes` names it, and `chain.py --check`'s only complaint was a
goal named by its number in this file, which this rewrite drops. This session claims it.

**The floor's `abi-probe` failure is load, not a regression** — see the playbook bullet.

## Next group

**Goal `bigint`, the gate** — one file set: `docs/agent/goals/69-bigint.toml`,
`crates/nvs-stdlib/src/ordering.rs`, `tests/conformance/core/`.

- [ ] **Re-run the goal's own acceptance list and claim it** — every check passed by hand this session,
      the ratchet gate included, and its manifest now names the ordering module and its rule. The
      stage-4 suite is `docs/agent/goals/69-bigint.toml:139` and the gate begins at
      `docs/agent/goals/69-bigint.toml:174`; `rule:classes/comparable` is what the gate pins.
- [ ] **Say in the doc comment whether a `bytes` pair is ever going to have a natural order** —
      `crates/nvs-stdlib/src/ordering.rs:46` calls it a one-line change here and one row in
      `crate::sort` "once a rule says so", while `rule:types/bytes` gives it none and
      `Core\Bytes::compare` is the spelling that does. Nothing is asking for the row; what is owed is
      the sentence naming that rule as the reason it stays out.

## Backlog

- The heap half of the object ordering is pinned in `heap-orders-by-comparable-or-by-its-comparator.nvst`
  and `heap-of-core-instants-needs-no-comparator.nvst`; the new case counts only its refusal.
- Goal `gap-zero` is next in the chain and is where the ratchet file itself goes —
  `docs/agent/goals/70-gap-zero.md`.
- `examples/bigint.nvs` prints the five figures the goal's acceptance list freezes; nothing else reads
  it — `docs/agent/goals/69-bigint.toml`.
