# Handoff

## State

Goal `core-io-1-2` (milestone dossier): `Core\IO::append`, `Core\IO::copy` and `Core\IO::canonicalize` have every feature proof, and `Core\IO` has its class card. `nvs.toml` grants `fs` read and write to the three `Core\IO` proof trees by `root`, so the next members need no config edit. The copy door in `crates/nvs-runtime/src/capability.rs` now refuses a copy of a file onto itself, which on Linux used to empty the file. Ten members remain; none is blocked.

## Next group

**Stage 1: `Core\IO` members, file order** — one file set: `crates/nvs-stdlib/src/io.rs` (registry rows, docs, bodies, `mod tests` with its `call_with`/`spelled` helpers), `nvs.toml`'s three `Core\IO` blocks already cover every path.
- [ ] **`Core\IO::exists`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:150`
- [ ] **`Core\IO::isDir`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:168`
- [ ] **`Core\IO::isFile`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:159`

## Backlog
- `Core\IO::isReadable`, `isWritable`, `lines`, `list`, `makeDir`, `modifiedAt`, `move` — the rest of `docs/agent/loop-goal.md`'s list, in that order.
- A copy onto itself on Windows throws the system's sharing-violation message, not the door's own sentence; both are an `IOError`. `crates/nvs-runtime/src/capability.rs`'s `same_file` says why.
