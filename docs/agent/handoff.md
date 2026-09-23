# Handoff

## State

Goal `core-io-2-2` is under way. `Core\IO::open`, `read`, `readText`, `remove`, `removeDir`,
`size`, `write`, `writeStream` and `stat` have every feature proof: the description, three
examples, an attack, a bench with its figure in `docs/perf/members.ndjson`, and a Rust `#[test]`
with its `covers:` marker in `crates/nvs-stdlib/src/io.rs`'s test module. Four members of the goal
are left: `walk`, `temporaryDir`, `within` and `stdin`.

## Next group

**Core\IO folder and scope members, one per slice** — one file set: `crates/nvs-stdlib/src/io.rs`
(the registry rows and the test module at its end) plus each member's new proof paths. Every proof
program under `docs/examples/core/IO`, `tests/hostile/core/IO` and `benches/members/core/IO` is
granted `fs.read` and `fs.write` everywhere by the root `nvs.toml`, so an attack must never name a
real file outside `Core\IO::temporaryDir()`. Run a proof from the repository root, or the root
`nvs.toml` is not found and every call throws for a missing capability. Take them in this order:

- [ ] **`Core\IO::walk`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:293`
- [ ] **`Core\IO::temporaryDir`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:308`
- [ ] **`Core\IO::within`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:333`
- [ ] **`Core\IO::stdin`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:385`

## Backlog

- A `cond ? 1 : 0` added to a `uint` does not compile (`E0407`, the literals are `int`); the
  `stat` bench uses an `if` instead. Not a finding against `Core\IO`; `rule:types/conversion` owns it.
