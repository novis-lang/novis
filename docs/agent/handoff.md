# Handoff

## State

Goal `core-io-1-2` (milestone dossier) is met: all 13 of its `Core\IO` members have every feature proof, and `python tools/dossier.py --verify --only` over them reports nothing owed, 39 examples ok and 13 attacks ok. `Core\IO::size` also has every proof, although it belongs to the next goal, `core-io-2-2`. `nvs.toml` grants `fs` read and write to the three `Core\IO` proof trees by `root`, so the next members need no config edit. The Rust tests in `crates/nvs-stdlib/src/io.rs`'s `mod tests` share `call_with`, `spelled`, `scratch`, `reading`, `writing`, `kinds_of`, `kinds_under`, `listed` and `lined`. `lined` reads an `Iterable<string>` value through `crate::instance::slot(answer.obj_ptr(), 0)`. Nothing is blocked.

## Next group

**Stage 1: `Core\IO` (2/2) members, file order** — one file set: `crates/nvs-stdlib/src/io.rs` (registry rows, docs, bodies, `mod tests` with its helpers); `nvs.toml`'s three `Core\IO` blocks already cover every path.

- [ ] **`Core\IO::read`** — owes whatever `python tools/dossier.py --id 'Core\IO::read'` prints. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:2005`
- [ ] **`Core\IO::readText`** — the same file set. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:2020`
- [ ] **`Core\IO::open`** — the same file set. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:2185`
- [ ] **`Core\IO::remove`** and **`removeDir`** — the same file set. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:3356`

## Backlog

- The rest of goal `core-io-2-2`: `stat`, `stdin`, `temporaryDir`, `walk`, `within`, `write`, `writeStream` — `docs/agent/goals/dossier/119-core-io-2-2.toml`.
