# Handoff

## State

Goal `core-bigint-1-2` has nine of its twelve members finished: `Core\BigInt::abs`, `add`,
`compareTo`, `div`, `mod`, `mul`, `format`, `gcd` and `lcm` each carry `about.md`, three examples,
one attack, one bench and a test from both sides. Three are left — `neg`, `of`, `ofUint`.

`gcd` and `lcm` share one Rust test, because they share one identity: their product is the magnitude
of their operands' product. They also share the `covers:` marker now on
`tests/conformance/core/bigint-number-theory-members-answer-their-own-identities.nvst`, which
asserted both identities all along and owed only the marker an instance member needs.
`format`'s Novis-side case is the radix round-trip case, credited by the call it makes.

The six benches this goal has written are not yet measured. `python tools/dossier.py --id` reports
them `stale` until the next sweep records a figure, which is the ordinary state of a new bench.

Nothing is blocked.

## Next group

**Stage: feature proofs for `Core\BigInt`** — one file set: `crates/nvs-stdlib/src/bigint.rs` for
the Rust-side test, `docs/examples/core/BigInt/<member>/`, `tests/hostile/core/BigInt/<member>/`,
`benches/members/core/BigInt/<member>.nvs`, and the conformance case that takes each `covers:`
marker. `rule:testing/feature-proofs` names what each one owes. `neg` is first because its Novis
side is already pinned in the number-theory case and owes only a marker; `of` and `ofUint` are the
two constructors and are neighbours in the implementing file. Every `.nvs` proof is cheapest to
write and bless **before** the slice's `crates/` edit — `docs/agent/playbook.md` § *Running things*
owns why.

- [ ] **`Core\BigInt::neg`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:240`
- [ ] **`Core\BigInt::of`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:121`
- [ ] **`Core\BigInt::ofUint`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:130`

## Backlog

- The six benches written in this goal have no figure yet; the `--record-perf` sweep writes them to
  `docs/perf/members.ndjson`.
- `Core\BigInt`'s twelve other members are goal `core-bigint-2-2`'s, behind this one in
  `docs/agent/goals/dossier/`.
