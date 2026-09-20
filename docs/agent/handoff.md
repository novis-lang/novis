# Handoff

## State

Goal `Core\Arr` (1/4). `Core\Arr::all` and `Core\Arr::any` are finished: `about.md`, three
examples each, one attack, one bench with a row in `docs/perf/members.ndjson`, and a Rust
`#[test]` carrying its `covers:` marker. Their Novis-side cases were already on disk, credited
by a plain call.

The `#[cfg(test)]` module at the foot of `crates/nvs-stdlib/src/arr.rs` now carries a closure
fixture — `list_of`, `below_ten`, `closure_of`, `walked` — so any further `Core\Arr` member
taking a callable gets its Rust proof in a few lines. `crates/nvs-stdlib/tests/allocation_policy.rs`
holds its own older copy of the same idea; neither is shared, because an integration test and a
unit test cannot see each other's helpers.

54 of `Core\Arr`'s features are still owed. Nothing is blocked.

## Next group

**One slice is one feature with all its feature proofs**, taken in file order so neighbours share
the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. All four are
the ends of a list, so one understanding of the edges pays for the group.

- [ ] **`Core\Arr::append`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:372`
- [ ] **`Core\Arr::prepend`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:384`
- [ ] **`Core\Arr::withoutFirst`** — owes examples, hostile, perf, tests;
      `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/arr.rs:396`
- [ ] **`Core\Arr::withoutLast`** — owes examples, hostile, perf, tests;
      `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/arr.rs:405`

## Backlog

- A closure call costs about three allocations an entry: `all` and `any` over four entries each
  measure 13 allocations and 344 bytes a round, and both rows are in `docs/perf/members.ndjson`.
  Whether `nvs_runtime::call_closure`'s argument slice can be reused across one walk is a perf
  question nobody owns.
- A member bench that calls a user closure can declare neither `allocations 0` nor `calls 0`, so
  `all` and `any` declare only `iterations`; `benches/members/README.md` § *What a bench declares*
  is the home if that should become a third shape.
- `Core\Arr` has 54 features still owed after this group.
