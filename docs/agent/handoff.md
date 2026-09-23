# Handoff

## State

Goal `core-io-1-2` (milestone dossier): `Core\IO::append`, `copy`, `canonicalize`, `exists`, `isFile`, `isDir`, `isReadable`, `isWritable`, `modifiedAt`, `move` and `makeDir` have every feature proof, and `Core\IO` has its class card. `Core\IO::size` has every proof too, one goal early: it is a member of goal `core-io-2-2`. `nvs.toml` grants `fs` read and write to the three `Core\IO` proof trees by `root`, so the next members need no config edit. The Rust tests in `crates/nvs-stdlib/src/io.rs`'s `mod tests` share `call_with`, `spelled`, `scratch`, `reading`, `writing`, `kinds_of` and `kinds_under`. The `makeDir` attack found that an empty path answered success with no folder created; `nvs_runtime::capability::create_dir` now throws `IOError` for it, pinned by `tests/conformance/core/io-make-dir-throws-for-an-empty-path.nvst`. Two members of this goal remain: `list` and `lines`; neither is blocked. Every `Core\IO` perf figure was re-measured this session.

## Next group

**Stage 1: `Core\IO` members, file order** — one file set: `crates/nvs-stdlib/src/io.rs` (registry rows, docs, bodies, `mod tests` with its helpers); `nvs.toml`'s three `Core\IO` blocks already cover every path.

- [ ] **`Core\IO::list`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:277`
- [ ] **`Core\IO::lines`** — owes examples, hostile, perf, tests. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/io.rs:359`

## Backlog

- `Core\IO::stat`, `remove`, `removeDir`, `walk`, `temporaryDir`, `within`, `readText`, `open`, `stdin` and the `Core\IO\File` members belong to goal `core-io-2-2`, not this one.
