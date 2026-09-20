# Handoff

## State

Goal `Core\Arr` (1/4). `lastKey`, `slice`, `replaceRange` and `chunk` are finished this session,
beside the 26 members that landed before them: `about.md`, three examples, one attack, one bench with
a row in `docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 30 of
`Core\Arr`'s 56 features are complete and 26 are owed. Nothing is blocked.

**`Core\Arr::count` answers `uint` while `slice`'s and `replaceRange`'s `offset` is `int`**, so a
count cannot be handed to a position — it is `E0401` at the call. Both a bench and an attack were
written that way and did not compile; the playbook's `uint` bullets are the family, and an `int`
counter carrying the position is the fix.

**A chunk attack repeating 20000 splits of a 100000-entry array ran 3m23s** against its declared
60s timeout, and 200 repeats assert the same thing in 2.2s. The playbook bullet has the shape.

The four figures, at a 2.9 to 3.1 ns calibration unit: `lastKey` 30.0 ns/op and 0 allocations, `slice`
69.7 and 2, `replaceRange` 119.1 and 4, and `chunk` 237.0 and 12 over 914 bytes, which is one array
per run plus the outer list. **Every `--record-perf` rebuilds the release binary**, so the four rows
name four different binaries while `impl_hash` stays `37de45ca70f4` across all of them — and the
ledger keys a figure's currency on that hash, not on the binary.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours share
the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. All four reshape an
array rather than reading one window of it, and what each `about.md` has to make plain is which keys
the result carries: `reverse` keeps them only when asked, `flip` turns every value into a key and so
loses an entry to every duplicate value, and the two `flatten` members renumber everything they pull
up. `reverse`'s Rust-side claim is already asserted by
`reverse_renumbers_by_default_and_keeps_every_key_on_request`, so its `covers:` marker is the whole
test edit.

- [ ] **`Core\Arr::reverse`** — owes examples, hostile, perf; its Rust test needs the `covers:` marker
      only, at `crates/nvs-stdlib/src/arr.rs:6587`; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:493`
- [ ] **`Core\Arr::flip`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:505`
- [ ] **`Core\Arr::flatten`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:514`
- [ ] **`Core\Arr::flattenDeep`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:523`

## Backlog

- `Core\Arr::column`, `sort`, `sortByKey`, `fill`, `fillKeys` and `range` are the group after that,
  same file set — `python tools/dossier.py --group 'Core\Arr' --owed` is the live list.
- `Core\Arr::sort` owes examples, perf and tests only: its attack is already on disk.
- The 20 members from `fromKeysAndValues` to `shapeAs` close the group; the whole class is then
  complete.
