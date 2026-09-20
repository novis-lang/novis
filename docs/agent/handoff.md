# Handoff

## State

Goal `Core\Arr` (1/4). `intersect`, `countBy` and `unique` are finished this session, beside the 47
members that landed before them: `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 50 of
`Core\Arr`'s 56 members are complete and 6 are owed. Nothing is blocked.

`countBy` owed only the marker. `count_by_counts_each_bucket_in_first_occurrence_order` already
asserted first-occurrence order and the `1`/`"1"` bucket, so the slice marked that test rather than
writing a second one. `intersect`'s new test is the complement of `diff`'s over the same subject, so
the two results together are the whole of `$a`; `unique`'s reads its mixed-tag result through a new
`length_of`, because `entries_of` panics on a value that is not a string.

At a 3.1 ns calibration unit `countBy` over a six-value list is 455.4 ns/op across 17 allocations,
`intersect` over a six-value list and a two-value list is 735.7 ns/op across 24.75, and `unique` over
a six-value list is 937.5 ns/op across 29. All three declare `calls 0` and the measurement agrees.
`intersect` sits beside `diff`'s 844.2 ns/op across 26, which is what says the two share one walk.

All three attacks reached their last step and stopped at the request's memory ceiling, and every
example prints what its comments claim. A value that cannot name a bucket is a `FATAL` out of
`countBy`, not a catchable throw, so an attack cannot `try`/`catch` one mid-file without ending the
program before its last step.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours share
the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. **Write every
`covers:` marker of the group before the first `--bless`**: a `crates/` edit makes the release binary
stale, and `--bless` then waits out a release build of `nvs-cli`.

- [ ] **`Core\Arr::min`** — owes `about.md`, three examples, an attack, a bench, and a `covers:`
      marker over a Rust `#[test]`; `rule:testing/feature-proofs`. It takes one argument and answers
      `?T`, so the empty array is the edge every example and the attack turn on, and the ordering is
      `compare_values` — the same one `sort` uses, never PHP's loose comparison.
      `crates/nvs-stdlib/src/arr.rs:5803`
- [ ] **`Core\Arr::max`** — owes the same five; `rule:testing/feature-proofs`. It is `min` with the
      other end of the same total order, so its arguments are built the same way.
      `crates/nvs-stdlib/src/arr.rs:5815`
- [ ] **`Core\Arr::sum`** — owes the same five; `rule:testing/feature-proofs`. It takes no `ctx`, and
      what its examples separate from `min`/`max` is that it answers a number over an empty array
      rather than `null`. `crates/nvs-stdlib/src/arr.rs:5828`

## Backlog

- `Core\Arr::product` and `Core\Arr::average` close this goal's numeric folds — `crates/nvs-stdlib/src/arr.rs:5839`, `:5858`.
- `Core\Arr::shapeAs` is the last member this goal owes and the only one taking four arguments; its Rust test builds them the way a call site does — `crates/nvs-stdlib/src/arr.rs:6169`.
- `Core\Arr` groups 2/4 to 4/4 follow this one — `docs/agent/goals/dossier/`.
- `serve::tests::a_mount_whose_unit_calls_url_absolute_and_resolves_no_origin_refuses_the_boot` fails side by side and passes alone; `verify.py` names it a shared-resource flake — `crates/nvs-cli/src/serve.rs:4049`.
