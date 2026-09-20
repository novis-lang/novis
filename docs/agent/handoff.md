# Handoff

## State

Goal `Core\Arr` (1/4). `Core\Arr::all`, `any`, `append` and `prepend` are finished: `about.md`,
three examples, one attack, one bench with a row in `docs/perf/members.ndjson`, and a Rust `#[test]`
carrying its `covers:` marker each.

`Core\Arr::append` panicked the whole process on a subject already holding `i64::MAX` as a key —
`NvsArray::append` asserts the next integer key is free, which a result copied from the subject does
not guarantee. It now throws the `LogicError` `$a[] = $v` throws, with that statement's message;
`crates/nvs-stdlib/src/arr.rs`'s `append_borrowed_or_refuse` is the seam and
`tests/conformance/core/arr-append-refuses-where-the-next-integer-key-is-taken.nvst` pins both forms
agreeing.

The bench that found that also priced `copy_entry`, which rendered a decimal key per entry for a
packed subject and handed it to a write that drops it. It now travels as a `nvs_runtime::SlotKey`
through `store_at`, which every member whose result starts as a copy of its subject shares:
`Core\Arr::append`'s figure fell from 10 allocations to 4 and from 203 ns/op to 121.

52 of `Core\Arr`'s features are still owed. Nothing is blocked.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours share
the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. The four are the
two ends of a list and the two paddings, so one understanding of an array's ends pays for the group.

- [ ] **`Core\Arr::withoutFirst`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:412`
- [ ] **`Core\Arr::withoutLast`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:421`
- [ ] **`Core\Arr::padStart`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:430`
- [ ] **`Core\Arr::padEnd`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:443`

## Backlog

- Every `Core\Arr` member whose result copies its subject now writes through
  `nvs_runtime::SlotKey`; their recorded figures predate that and are re-measured as each is
  proved — `docs/perf/members.ndjson`.
- 52 `Core\Arr` features still owe their feature proofs — `python tools/dossier.py --group 'Core\Arr'`.
