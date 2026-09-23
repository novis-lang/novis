# Handoff

## State

Goal `core-io-1-2` (milestone dossier): `Core\IO::append`, `copy`, `canonicalize`, `exists`, `isFile`, `isDir`, `isReadable` and `isWritable` have every feature proof, and `Core\IO` has its class card. `Core\IO::size` has every proof too, one goal early: it is a member of goal `core-io-2-2`, not of this one. `nvs.toml` grants `fs` read and write to the three `Core\IO` proof trees by `root`, so the next members need no config edit. The Rust tests share two fixtures in `crates/nvs-stdlib/src/io.rs`'s `mod tests`: `kinds_of` (a file, a directory and a missing name under `fs.read`) and `kinds_under` (the same under a context the caller builds). Five members of this goal remain: `modifiedAt`, `move`, `makeDir`, `list`, `lines`; none is blocked. A Rust test edit to `io.rs` did not make the other `Core\IO` figures stale: `dossier.py --record-perf --group 'Core\IO'` found them current.

## Next group

**Stage 1: `Core\IO` members, file order** — one file set: `crates/nvs-stdlib/src/io.rs` (registry rows, docs, bodies, `mod tests` with its `call_with`/`spelled`/`kinds_of`/`kinds_under` helpers); `nvs.toml`'s three `Core\IO` blocks already cover every path.

- [ ] **`Core\IO::modifiedAt`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:208`
- [ ] **`Core\IO::move`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:241`
- [ ] **`Core\IO::makeDir`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:259`

## Backlog

- `Core\IO::list` and `Core\IO::lines` are the last two members of this goal, after the group above — `docs/agent/loop-goal.md`.
- `dossier.py --bless` builds `target/release/nvs.exe` first when it is older than the tree, which takes over two minutes; run it in the background — `tools/dossier.py`.
