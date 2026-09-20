# Handoff

## State

Goal `Core\Arr` (1/4). `sort` and `fill` are finished this session, beside the 36 members that
landed before them: `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 38 of
`Core\Arr`'s 56 members are complete and 18 are owed. Nothing is blocked.

Neither member owed a Rust test that was not already written. `sort` had seven `#[test]`s over
`nvs_core_arr_sort` and `fill` one, all of them unattributed, so that half of both slices was the
`covers:` marker and nothing else. At a 2.8 ns calibration unit `sort` is 188.0 ns/op over 8
allocations for a six-value list, and `fill` 95.8 ns/op over 2 for eight cells; both benches
declare `calls 0` and the measurement agrees.

`Core\Arr::fill` at a count of four billion reaches the request's memory ceiling, which is a
`FATAL` no `try` catches, so its attack puts that step last and declares `// hostile: ends-early`.
That is the ceiling working, not a gap; the playbook holds the shape it forces on an attack.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. The first two
below already have their Rust tests written, so the Rust half of each is one marker line. **Write
every marker of the group before the first `--bless`**: a `crates/` edit makes the release binary
stale, and `--bless` then waits out a three-minute relink.

- [ ] **`Core\Arr::fillKeys`** — owes `about.md`, three examples, an attack, a bench, and the
      `covers:` marker over `fill_keys_stores_one_value_under_each_distinct_key` and
      `filling_under_a_key_that_is_not_a_key_throws`; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:578`
- [ ] **`Core\Arr::range`** — owes the same five; its Rust side is the block the `range_of` helper
      drives. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/arr.rs:587`
- [ ] **`Core\Arr::fromKeysAndValues`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:595`

## Backlog

- `docs/examples/core/Arr/sort/01-values-and-order.nvs` has no top comment and `02` joins two
  clauses with a dash; goal `plain-comments` owns that sweep, not this goal.
- `tests/hostile/core/Arr/sort/01-every-row-at-size-with-its-corners.nvs` opens with a 60-word
  sentence, for the same sweep.
