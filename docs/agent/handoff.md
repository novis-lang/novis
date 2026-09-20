# Handoff

## State

Goal `core-bigint-1-2` has three of its twelve members finished: `Core\BigInt::abs`, `add` and
`compareTo` each carry `about.md`, three examples, one attack, one bench figure and a test from both
sides. Nine are left — `div`, `format`, `gcd`, `lcm`, `mod`, `mul`, `neg`, `of`, `ofUint`.

`crates/nvs-stdlib/src/bigint.rs`'s module doc claimed a sort over these values takes
`{comparator: ...}`. It does not: `crate::ordering` sends two objects to `Comparable`'s `compareTo`,
`tests/conformance/core/bigint-orders-through-compare-to-and-every-spelling-of-it-agrees.nvst:59`
already asserted the bare `Core\Arr::sort`, and the sentence is corrected rather than reconciled.

Nothing is blocked.

## Next group

**Stage: feature proofs for `Core\BigInt`** — one file set: `crates/nvs-stdlib/src/bigint.rs` for the
Rust-side test and the module doc, `docs/examples/core/BigInt/<member>/`,
`tests/hostile/core/BigInt/<member>/`, `benches/members/core/BigInt/<member>.nvs`, and the
`tests/conformance/core/bigint-*.nvst` case that takes each `covers:` marker.
`rule:testing/feature-proofs` names what each one owes; the three below are neighbours in the
implementing file and share one conformance case.

- [ ] **`Core\BigInt::div`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:176`
- [ ] **`Core\BigInt::mod`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:185`
- [ ] **`Core\BigInt::mul`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:167`

## Backlog

- `Core\BigInt::format`, `gcd`, `lcm`, `neg`, `of` and `ofUint` — the rest of goal
  `core-bigint-1-2`, `docs/agent/goals/dossier/95-core-bigint-1-2.md`.
- The half of the class in goal `core-bigint-2-2`, `docs/agent/goals/dossier/96-core-bigint-2-2.md`.
- `docs/perf/members.md` is rendered from the ledger by `python tools/dossier.py --perf-report`, and
  no slice here regenerates it.
