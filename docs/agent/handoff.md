# Handoff

## State

Goal `Core\Arr` (1/4). `find`, `findKey`, `isEmpty` and `hasKey` are finished, beside `count`,
`filter`, `map`, `mapKeys`, `groupBy`, `reduce`, `any` and `all`: `about.md`, three examples, one
attack, one bench with a row in `docs/perf/members.ndjson`, and a Rust `#[test]` carrying its
`covers:` marker each. 38 of `Core\Arr`'s features are still owed. Nothing is blocked.

The four figures, at a 3.0 to 3.1 ns calibration unit and binary `18d233f52746`: `find` 195.3 ns/op
and 10 allocations, `findKey` 148.2 and 4, `hasKey` 29.7 and 1, `isEmpty` 7.8 and 0. The whole
group's ledger rows were re-measured at that binary, which is the playbook bullet above.

`crates/nvs-stdlib/src/arr.rs` `# Known gaps` 2 and 3 are new, both owner M12 and both found by a
bench. 2: a walk builds its callback's key argument whether the callback declares one or not, so
`benches/members/core/Arr/find.nvs` counts 10 allocations per operation over a four-entry list
against 4 for the same program over a four-entry string-keyed array. 3: `key_bytes` copies its key
onto the heap per call, so `hasKey` counts one allocation per operation while neither walking nor
calling back.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. The first two
search the values rather than the keys, so `find`'s and `findKey`'s landed proofs are the shape to
follow and `hasKey`'s `about.md` is what each has to be told apart from; the last two read the
array's shape and take no second argument at all.

- [ ] **`Core\Arr::contains`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:293`
- [ ] **`Core\Arr::keyOf`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:302`
- [ ] **`Core\Arr::isList`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:311`
- [ ] **`Core\Arr::keys`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:320`

## Backlog

- `Core\Arr::values`, `first`, `last`, `firstKey`, `lastKey` are the group after that, same file set.
- Re-record the group's ledger rows as the last step of any session that edits `arr.rs`
  (`docs/agent/playbook.md` § Tooling).
- `Core\Arr::average` is the check the driver reports; it is owed like the other 38.
