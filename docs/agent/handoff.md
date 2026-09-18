# Handoff

## State

**Goal `bigint` — `Core\BigInt` — is registered and its whole roster is on disk.**
`crates/nvs-stdlib/src/bigint.rs` holds all 24 members of the goal's § *Stage 3* with their
reference cards, bodies and `address` arms; `python tools/verify.py` is green over it.

**Every `spec-*-outstanding.txt` ratchet under `crates/nvs-stdlib/tests/` now holds zero keys** —
striking `§13 Core\BigInt` emptied the last one, so the spec's roster is registered whole. Stage 0's
catch-up landed with it: `rule:core-api/tier-roster`'s *Designed, not shipped* paragraph no longer
names classes that are registered, and `docs/rules/core-api.md` is its render.

**Six conformance cases under `tests/conformance/core/bigint-*.nvst`** carry the class past the
floor of three: the `div`/`mod` sign sweep, the bound on both sides of `int`, `uint` and `decimal`,
the radix round trip, every refusal with its message frozen, the ordering agreement, and the
number-theory identities.

**One thing the goal's prose promised that the tree does not do:** `Core\Arr::sort` does not order
two `Comparable` objects — see the playbook bullet and the next group's third item. The operators
and `compareTo` do, and the cases assert them that way.

## Next group

**Goal `bigint`, stage 4 — the proofs** — one file set: `examples/bigint.nvs` (new),
`docs/agent/goals/69-bigint.toml`, `crates/nvs-stdlib/src/ordering.rs`.

- [ ] **`examples/bigint.nvs`, with the output the acceptance list freezes** — the goal's
      § *Stage 4* at `docs/agent/goals/69-bigint.md:64` names the five things it must print, and
      `.agent-tmp` is not where a fixture goes. `rule:core-api/shape-rules`.
- [ ] **The guard tests the acceptance list names** — read them off
      `docs/agent/goals/69-bigint.toml:1` first, then grep each name across `crates/`: the six
      `.nvst` cases already landed may hold the claim under another name, which the playbook's
      *A `loop-goal.toml` check's drafted `tests` list can be mixed* bullet is about.
- [ ] **Decide `Core\Arr::sort` over a `Comparable` object** — `crates/nvs-stdlib/src/ordering.rs:36`
      is the doc paragraph that declines it, `crates/nvs-stdlib/src/instance.rs:185` is the
      `ClassDesc::comparer` that would answer it, and `crates/nvs-runtime/src/dispatch.rs:1` is how
      native code reaches a member by name. Either build the object row — which closes the gap for
      every `Core` class with a `compareTo`, not just this one — or correct the goal's § *Stage 2*
      sentence that lists `Core\Arr::sort` beside `echo` and `<=>`. `rule:classes/comparable` covers
      the operators only.

## Backlog

- **The `_` separator divergence** — `Core\BigInt::parse` refuses `"1_000"` because `"1_000" as int`
  does, though an integer *literal* may carry it; `crates/nvs-stdlib/src/bigint.rs`'s `parse` body
  owns the reasoning, and no rule states it.
- **`MAX_BITS` is this module's bound, not a rule's** — `crates/nvs-stdlib/src/bigint.rs:107`; ADR
  0054 left it open and only `pow` and `shl` are checked against it.
- **`Core\BigDecimal`** stays unscheduled — ADR 0054 § 6 names it, no spec row asks for it
  (`docs/agent/goals/69-bigint.md:91`).
