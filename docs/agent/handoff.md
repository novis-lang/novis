# Handoff

## State

Goal `core-bigint-2-2` — `Core\BigInt` (2/2), twelve features. `parse`, `pow` and `powMod` now
carry every feature proof: `about.md`, three examples, one attack, one bench and a Rust-side test.
The other nine are open and all live in one file, `crates/nvs-stdlib/src/bigint.rs`.

`python tools/dossier.py --record-perf` re-measured all fifteen `Core\BigInt` benches, because the
module's text moved. Nothing is blocked.

## Next group

**`Core\BigInt`'s two shifts and its sign** — one file set: `crates/nvs-stdlib/src/bigint.rs` for
the registry row, the member and the Rust test, plus `docs/examples/core/BigInt/<member>/`,
`tests/hostile/core/BigInt/<member>/` and `benches/members/core/BigInt/<member>.nvs`. One slice is
one feature with all of `rule:testing/feature-proofs`'s proofs; `python tools/dossier.py --id
'<feature>'` prints the path each belongs at. `shl` is checked against `MAX_BITS` the way `pow` is,
so its attack is the one `pow`'s attack already showed the shape of.

- [ ] **`Core\BigInt::shl`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/bigint.rs:278`
- [ ] **`Core\BigInt::shr`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/bigint.rs:287`
- [ ] **`Core\BigInt::sign`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/bigint.rs:296`

## Backlog

- The rest of this goal's list, in its order: `sqrt`, `sub`, `toDecimal`, `toInt`, `toString`,
  `toUint` — `docs/agent/loop-goal.md` § *The item list*.
- Which answer `powMod` owes for a negative receiver — `crates/nvs-stdlib/src/bigint.rs`
  § *Known gaps* 1, owner M11.
