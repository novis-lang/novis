# Handoff

## State

Goal `core-io-2-2` is under way. `Core\IO::open`, `read`, `readText`, `remove`, `removeDir` and
`size` have every feature proof: the description, three examples, an attack, a bench with its
figure in `docs/perf/members.ndjson`, and a Rust `#[test]` with its `covers:` marker in
`crates/nvs-stdlib/src/io.rs`'s test module. Seven members of the goal are left: `write`,
`writeStream`, `stat`, `walk`, `temporaryDir`, `within` and `stdin`.

## Next group

**Core\IO writing members, one per slice** — one file set: `crates/nvs-stdlib/src/io.rs` (the
registry rows and the test module at its end) plus each member's new proof paths. Every proof
program under `docs/examples/core/IO`, `tests/hostile/core/IO` and `benches/members/core/IO` is
granted `fs.read` and `fs.write` everywhere by the root `nvs.toml`, so an attack must never name a
real file outside `Core\IO::temporaryDir()`. Take them in this order:

- [ ] **`Core\IO::write`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:108`
- [ ] **`Core\IO::writeStream`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:132`
- [ ] **`Core\IO::stat`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:220`

## Backlog

- `Core\IO::walk`, `temporaryDir`, `within` and `stdin` owe every proof but the description — `python tools/dossier.py --owed --only 'Core\IO::walk'`; rows at `crates/nvs-stdlib/src/io.rs:293`, `:308`, `:333`, `:385`.
- `REMOVE_DOC`'s return line ("throws rather than answering quietly") is not yet in the voice of `AGENTS.md` § *Text an end user reads* — goal `core-class-cards` owns the cards.
