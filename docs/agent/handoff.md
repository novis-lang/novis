# Handoff

## State

Goal `Core\Arr` (1/4). `reverse`, `flip`, `flatten` and `flattenDeep` are finished this session,
beside the 30 members that landed before them: `about.md`, three examples, one attack, one bench
with a row in `docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each.
34 of `Core\Arr`'s features are complete and 21 are owed. Nothing is blocked.

**The acceptance check that was red at session start was the stale release binary, not owed work.**
`python tools/dossier.py --bless` refuses with `target/release/nvs.exe is missing or older than the
tree` and writes nothing; `cargo build --release -p nvs-cli` by hand cleared it. Every `crates/`
edit of a group belongs before the first `--bless` or `--record-perf`, because each one costs
another six-minute relink, and this session paid it twice. The playbook bullet now says both.

The four figures, at a 2.9 ns calibration unit: `flattenDeep` 129.8 ns/op over 6 allocations,
`flatten` 139.0 over 5, `reverse` 164.4 over 7, and `flip` 682.6 over 33 allocations and 1300 bytes
for a six-entry subject. `flip` is the most expensive of the four by a factor of four, and why has
not been checked.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. None of the four
has a Rust-side test carrying its marker today, so each slice writes one as well. What each
`about.md` has to make plain is where the result's entries come from: `column` pulls one key out of
every row, `sortByKey` puts the entries in the order of their keys, and `fill` and `fillKeys` build
an array out of a value that is repeated.

- [ ] **`Core\Arr::column`** — owes `about.md`, examples, hostile, perf and a Rust test with its
      `covers:` marker; `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/arr.rs:532`
- [ ] **`Core\Arr::sortByKey`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:545`
- [ ] **`Core\Arr::fill`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:569`
- [ ] **`Core\Arr::fillKeys`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:578`

## Backlog

- `Core\Arr::flip` costs 682.6 ns/op and 33 allocations over a six-entry array — the highest per
  entry of the four measured here, and not investigated; `docs/perf/members.ndjson`.
- The 17 features after the next group, in registry order from `range` onward;
  `python tools/dossier.py --id 'Core\Arr::<name>'` per member.
- `Core\Arr::range` already has a Rust test and no marker, so its `tests` proof is one line:
  `range_matches_phps_ascending_descending_and_stepped_forms` in `crates/nvs-stdlib/src/arr.rs`.
