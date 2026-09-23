# Handoff

## State

Goal `core-io-2-2` is under way. `Core\IO::open`, `read`, `readText`, `remove`, `removeDir`,
`size`, `write`, `writeStream`, `stat`, `walk` and `temporaryDir` have every feature proof: the
description, three examples, an attack, a bench with its figure in `docs/perf/members.ndjson`, and
a Rust `#[test]` with its `covers:` marker in `crates/nvs-stdlib/src/io.rs`'s test module. Two
members of the goal are left: `within` and `stdin`.

A full `dossier.py --run all` sweep, run while the machine was busy, timed out three unrelated
hostile cases (`core/Arr/diff`, `core/Arr/intersect`, `core/Html/sanitize/01`). Each ran eight at
a time; nothing this goal touched is among them.

## Next group

**Core\IO scope members, one per slice** — one file set: `crates/nvs-stdlib/src/io.rs` (the
registry rows and the test module at its end) plus each member's new proof paths. Every proof
program under `docs/examples/core/IO`, `tests/hostile/core/IO` and `benches/members/core/IO` is
granted `fs.read` and `fs.write` everywhere by the root `nvs.toml`, so an attack must never name a
real file outside `Core\IO::temporaryDir()`. Run a proof from the repository root, or the root
`nvs.toml` is not found and every call throws for a missing capability. An example that prints a
path normalizes `Core\Path::SEPARATOR` to `/`, or its `.out` differs between Windows and WSL.
Take them in this order:

- [ ] **`Core\IO::within`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:333`
- [ ] **`Core\IO::stdin`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:385`

## Backlog

- None of this goal's members is left without an owner; `within` and `stdin` above are the goal's
  whole remainder (`python tools/dossier.py --id 'Core\IO::within'`).
