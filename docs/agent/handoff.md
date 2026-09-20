# Handoff

## State

Goal `core-bigint-1-2` has all twelve of its members carrying the feature proofs
`rule:testing/feature-proofs` names: `abs`, `add`, `compareTo`, `div`, `format`, `gcd`, `lcm`,
`mod`, `mul`, `neg`, `of` and `ofUint` each have `about.md`, three examples, one attack, one bench,
a Novis-side case and a Rust `#[test]`.

`neg` is credited from Novis by the div/mod sign sweep case, which flips a sign twice, keeps the
magnitude and prints a flipped zero — an instance member owes a `covers:` marker there and nothing
else. `of` and `ofUint` are static, so the cases that call them credit them already, and each owed
only a Rust test.

The figures were recorded at the end of the group rather than per slice, because every slice moved
`crates/nvs-stdlib/src/bigint.rs` and `rule:testing/member-perf-ledger` re-measures a member when
its implementing file does.

Nothing is blocked.

## Next group

**Stage: feature proofs for `Core\BigInt`, the second half** — one file set:
`crates/nvs-stdlib/src/bigint.rs` for the Rust-side test, `docs/examples/core/BigInt/<member>/`,
`tests/hostile/core/BigInt/<member>/`, `benches/members/core/BigInt/<member>.nvs`, and the
conformance case that takes each `covers:` marker. `rule:testing/feature-proofs` names what each one
owes. Write and bless every `.nvs` proof **before** the group's `crates/` edit, and record the
figures once at the end — `docs/agent/playbook.md` § *Running things* prices each relink.

- [ ] **`Core\BigInt::parse`** — owes about, examples, hostile, perf and a Rust test; its Novis side
      is the radix round-trip case. `crates/nvs-stdlib/src/bigint.rs:138`
- [ ] **`Core\BigInt::sub`** — owes about, examples, hostile, perf and a Rust test.
      `crates/nvs-stdlib/src/bigint.rs:158`
- [ ] **`Core\BigInt::sign`** — owes about, examples, hostile, perf and a Rust test.
      `crates/nvs-stdlib/src/bigint.rs:275`

## Backlog

- The other eleven members of `Core\BigInt` — goal `core-bigint-2-2`, `docs/agent/goals/dossier/`.
- A `.nvs` proof is reformatted by `verify.py`'s `nvs-fmt` step, which rewrites `"` to `'` for a
  string with nothing to interpolate; write it either way and let the step settle it.
