# Handoff

## State

Goal `Core\Arr` (1/4). `Core\Arr::withoutFirst`, `withoutLast`, `padStart` and `padEnd` are
finished: `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 48 of
`Core\Arr`'s features are still owed. Nothing is blocked.

`Core\Arr::withoutFirst` costs 11 allocations and 306 ns/op where `withoutLast` costs 2 and 123,
over the same four-entry subject. That is the price of the rule that every surviving key is kept:
the result starts at key `"1"`, and `crates/nvs-stdlib/src/arr.rs:2763` states that a gap is exactly
what degrades a result to the hash form, which renders every key as a string. It is the documented
behaviour rather than a bug, and the only fix is a packed array that carries a base offset — a
representation change, larger than a slice.

`Core\Arr::all` and `any` owed nothing but a re-measurement, and their rows are re-recorded. That
run's calibration unit came out at 25.1 ns against the 3.0 the four new rows were taken at, so their
deterministic counts stand and their `ns/op` is noise.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. The last three
are the callback family, so one understanding of what a callback sees and what happens to the keys
pays for the group.

- [ ] **`Core\Arr::count`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:96`
- [ ] **`Core\Arr::filter`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:105`
- [ ] **`Core\Arr::map`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:117`
- [ ] **`Core\Arr::mapKeys`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:129`

## Backlog

- `Core\Arr::all` and `any`'s newest `ns/op` rows were measured on a loaded machine; re-record them
  with `--force` when it is idle — `docs/perf/members.ndjson`.
- A packed array carrying a base offset would give `Core\Arr::withoutFirst` `withoutLast`'s cost —
  `crates/nvs-runtime/src/array.rs`, and `crates/nvs-stdlib/src/arr.rs:2763` is the measurement.
- 48 `Core\Arr` features still owe their feature proofs — `python tools/dossier.py --group
  'Core\Arr' --owed` is the worklist.
