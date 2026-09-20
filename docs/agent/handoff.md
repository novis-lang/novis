# Handoff

## State

Goal `Core\Arr` (1/4). `values`, `first`, `last` and `firstKey` are finished, beside `contains`,
`keyOf`, `isList`, `keys`, `find`, `findKey`, `isEmpty`, `hasKey`, `count`, `filter`, `map`,
`mapKeys`, `groupBy`, `reduce`, `any` and `all`: `about.md`, three examples, one attack, one bench
with a row in `docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each.
30 of `Core\Arr`'s features are still owed. Nothing is blocked.

**No new Rust test was written this session, because all four were already asserted and only the
attribution was missing.** `values_renumbers_from_zero_and_retains_what_it_copies`
(`crates/nvs-stdlib/src/arr.rs:6471`), `the_four_end_members_read_insertion_order` (`:7389`) and
`an_empty_array_has_no_ends` (`:7404`) carry the markers now, and the two end ones name
`Core\Arr::lastKey` as well — so `lastKey` owes examples, an attack and a bench only.

**A bench reaching its array through a nested index charged the member two allocations per op**, and
the declared `// bench: allocations 0` is what caught it; the playbook bullet has the shape. The four
figures, at a 2.6 to 3.2 ns calibration unit and binary `af29429c0f73`: `first` 14.0 ns/op and 0
allocations, `last` 11.3 and 0, `firstKey` 29.4 and 0 — 59.4 and 2 through the nested index — and
`values` 97.9 and 4, which is its sibling `keys`'s shape.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours share
the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. `lastKey` is
`firstKey`'s other end and its Rust proof already landed, so its `about.md` is what has to tell the
two apart. The other three take a range and return a new array, and what each `about.md` has to make
plain is what happens at the ends of that range and which keys the result carries.

- [ ] **`Core\Arr::lastKey`** — owes examples, hostile, perf; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:375`
- [ ] **`Core\Arr::slice`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:384`
- [ ] **`Core\Arr::replaceRange`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:398`
- [ ] **`Core\Arr::chunk`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:412`

## Backlog

- 30 `Core\Arr` features still owe their proofs; `python tools/dossier.py --group 'Core\Arr'` is the
  list, thinnest first.
- `crates/nvs-stdlib/src/arr.rs` `# Known gaps` 4, owner M12: a member that builds an array grows its
  storage as it appends with the entry count already in hand.
