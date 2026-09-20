# Handoff

## State

Goal `Core\Arr` (1/4). `Core\Arr::mapKeys`, `groupBy` and `reduce` are finished, beside `count`,
`filter` and `map`: `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 42 of
`Core\Arr`'s features are still owed. Nothing is blocked.

The three figures, at the same 3.0 to 3.4 ns calibration unit as the rest of the group and over a
three-entry subject: `mapKeys` 605.8 ns/op and 25 allocations, `groupBy` 916.1 and 41, `reduce`
139.5 and 5, against `map`'s 176 and 6. Every member that calls back into Novis pays one heap
vector per call (`nvs-runtime` `# Known gaps` 10), which is the playbook bullet above and why only a
callback-free member declares `allocations 0`.

`crates/nvs-runtime/src/lib.rs` `# Known gaps` 12 is new, found while reading what the attacks' last
step prints: `Limits::memory_breach` reports `memory_used()` at the poll, so a program refused its
next doubling while holding a 134,217,728-character text says `18825 bytes held against a ceiling of
268435456`. Owner M10.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. The first two
take a predicate and stop at the first match, so `all`'s and `any`'s landed proofs are the shape to
follow; the last two take no callback at all and are the cheap pair that fits behind them.

- [ ] **`Core\Arr::find`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:200`
- [ ] **`Core\Arr::findKey`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:212`
- [ ] **`Core\Arr::isEmpty`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:248`
- [ ] **`Core\Arr::hasKey`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:257`

## Backlog

- The memory-limit `FATAL`'s reading: `crates/nvs-runtime/src/lib.rs` `# Known gaps` 12, owner M10.
- A callback appending to the property its member is walking: `crates/nvs-stdlib/src/arr.rs`
  `# Known gaps` 1, owner M11, and its fix is a helper convention the user has not been asked about.
- `docs/examples/core/Arr/sort` holds examples from an earlier pass and no `about.md`; it comes up
  as a slice in registry order rather than as work of its own.
