# Handoff

## State

Goal `core-io-1-2` (milestone dossier): `Core\IO::append`, `copy`, `canonicalize`, `exists`, `isFile` and `isDir` have every feature proof, and `Core\IO` has its class card. `nvs.toml` grants `fs` read and write to the three `Core\IO` proof trees by `root`, so the next members need no config edit. The Rust tests for the three predicates share one fixture, `kinds_of` in `crates/nvs-stdlib/src/io.rs`'s `mod tests`, which answers a file, a directory and a missing name in that order. Seven members remain; none is blocked.

## Next group

**Stage 1: `Core\IO` members, file order** — one file set: `crates/nvs-stdlib/src/io.rs` (registry rows, docs, bodies, `mod tests` with its `call_with`/`spelled`/`kinds_of` helpers), `nvs.toml`'s three `Core\IO` blocks already cover every path.

- [ ] **`Core\IO::isReadable`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:178`
- [ ] **`Core\IO::isWritable`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:187`
- [ ] **`Core\IO::size`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:196`

## Backlog

- `nvs agent find "delete file"` finds nothing: `Core\IO::remove`'s card never says "delete", so a user who types that word does not reach it. Owner: `crates/nvs-stdlib/src/io.rs` `REMOVE_DOC`, when `remove` takes its proofs.
