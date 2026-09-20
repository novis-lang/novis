# Handoff

## State

Goal `core-bigint-2-2` — `Core\BigInt` (2/2), twelve features. `parse`, `pow`, `powMod`, `shl`,
`shr` and `sign` now carry every feature proof: `about.md`, three examples, one attack, one bench
and a Rust-side test. Six are open — `sqrt`, `sub`, `toDecimal`, `toInt`, `toString`, `toUint` —
and all of them live in one file, `crates/nvs-stdlib/src/bigint.rs`.

`python tools/dossier.py --record-perf --group 'Core\BigInt'` measured the three new benches; the
other fifteen keep their figures, since no implementation moved. Nothing is blocked.

## Next group

**`Core\BigInt`'s two remaining arithmetic members** — one file set:
`crates/nvs-stdlib/src/bigint.rs` for the registry row, the member and the Rust test, plus
`docs/examples/core/BigInt/<member>/`, `tests/hostile/core/BigInt/<member>/` and
`benches/members/core/BigInt/<member>.nvs`. One slice is one feature with all of
`rule:testing/feature-proofs`'s proofs; `python tools/dossier.py --id '<feature>'` prints the path
each belongs at. `sqrt` refuses a negative receiver, so its attack has a refusal to name the way
`pow`'s attack named the width bound; `sub` refuses nothing and its attack is width and volume
alone.

- [ ] **`Core\BigInt::sub`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/bigint.rs:179`
- [ ] **`Core\BigInt::sqrt`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/bigint.rs:233`
- [ ] **`Core\BigInt::toString`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/bigint.rs:341`

## Backlog

- `toDecimal`, `toInt` and `toUint` are the last three of the goal, and share the same file set — `docs/agent/loop-goal.md`.
- Every `Core\BigInt` bench declares `calls 0` and 200000 iterations; `sign` takes 400000 — `benches/members/README.md`.
- Blessing after a Rust edit pays a release build, so every Rust edit goes first — `docs/agent/playbook.md`.
