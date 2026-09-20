# Handoff

## State

Goal `core-bigint-1-2` has six of its twelve members finished: `Core\BigInt::abs`, `add`,
`compareTo`, `div`, `mod` and `mul` each carry `about.md`, three examples, one attack, one bench and
a test from both sides. Six are left — `format`, `gcd`, `lcm`, `neg`, `of`, `ofUint`.

`div` and `mod` take their Rust-side proof from the sign sweep that already asserted both roundings
(`crates/nvs-stdlib/src/bigint.rs:1203`); it owed a `covers:` marker and nothing else. `mul` has its
own test there, and the factorial case carries its `.nvst` marker.

The three new benches are written but not yet measured. `python tools/dossier.py --id` reports them
`stale` until the next sweep records a figure, which is the ordinary state of a new bench.

Nothing is blocked.

## Next group

**Stage: feature proofs for `Core\BigInt`** — one file set: `crates/nvs-stdlib/src/bigint.rs` for
the Rust-side test, `docs/examples/core/BigInt/<member>/`, `tests/hostile/core/BigInt/<member>/`,
`benches/members/core/BigInt/<member>.nvs`, and the `tests/conformance/core/bigint-*.nvst` case that
takes each `covers:` marker. `rule:testing/feature-proofs` names what each one owes. `format` is
first because the driver's acceptance check names it; `gcd` and `lcm` are neighbours in the
implementing file and share
`tests/conformance/core/bigint-number-theory-members-answer-their-own-identities.nvst`.

- [ ] **`Core\BigInt::format`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:330`
- [ ] **`Core\BigInt::gcd`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:222`
- [ ] **`Core\BigInt::lcm`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:231`

## Backlog

- `Core\BigInt::neg`, `of` and `ofUint` are the rest of this goal — `python tools/dossier.py --id`.
- A `--bless` run costs a six-minute release relink after any `crates/` edit, so the group's Rust
  edits go in first — `docs/agent/playbook.md` § *Running things*.
