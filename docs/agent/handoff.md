# Handoff

## State

Goal `Core\Arr` (1/4). `contains`, `keyOf`, `isList` and `keys` are finished, beside `find`,
`findKey`, `isEmpty`, `hasKey`, `count`, `filter`, `map`, `mapKeys`, `groupBy`, `reduce`, `any` and
`all`: `about.md`, three examples, one attack, one bench with a row in `docs/perf/members.ndjson`,
and a Rust `#[test]` carrying its `covers:` marker each. 34 of `Core\Arr`'s features are still owed.
Nothing is blocked.

**Two benches found an allocation the member did not need, and both are fixed rather than
recorded.** `nvs_runtime::value_identical` seeded its worklist with `vec![(left, right)]`, so every
comparison over two scalars allocated — `==` on the request path as much as this group;
`crates/nvs-runtime/src/identity.rs:136` compares the first pair before the vector is touched.
`nvs_stdlib::arr::is_list` asked `key_at` per entry, which renders a packed array's position into a
fresh `NvsStr`; it reads `slot_key` and compares against a stack-rendered spelling.

The four figures, at a 2.7 ns calibration unit and binary `bee8b7ffeaf6`: `contains` 23.6 ns/op and
0 allocations, `keyOf` 27.6 and 0, `isList` 10.5 and 0 — 72.9 and 4 before the fix, at a 3.2 ns unit
— and `keys` 99.3 and 4. Every other `Core\Arr` row was re-measured at that binary, because
`arr.rs`'s text moved.

`crates/nvs-stdlib/src/arr.rs` `# Known gaps` 4 is new, owner M12 and found by a bench: a member
that builds an array grows its storage as it appends with the entry count already in hand, because
`NvsArray` offers `new` and no way to reserve.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. `values` is
`keys`'s other half, so `keys`'s landed proofs are the shape to follow and its `about.md` is what
`values` has to be told apart from. The other three read one entry off an end of the array, answer
`null` over the empty array, and differ only in which end and in value against key — which is the
one thing each `about.md` has to make plain.

- [ ] **`Core\Arr::values`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:339`
- [ ] **`Core\Arr::first`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:348`
- [ ] **`Core\Arr::last`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:357`
- [ ] **`Core\Arr::firstKey`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:366`

## Backlog

- `Core\Arr::lastKey` closes the end-reading family; `crates/nvs-stdlib/src/arr.rs:375`.
- `arr.rs` `# Known gaps` 2 and 3 are still open, both owner M12: the key argument a walk builds for
  a callback that never declared one, and `key_bytes`'s heap copy per call.
- `arr.rs` `# Known gaps` 4, owner M12: a capacity constructor on `NvsArray`, which `filter` at 16
  allocations, `mapKeys` at 25 and `groupBy` at 41 would each then be read against.
- `docs/perf/members.md` is generated from the ledger by `dossier.py --perf-report` and has not been
  regenerated since 2026-09-17; no goal owns doing it.
