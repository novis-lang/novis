# Handoff

## State

Goal `core-io-file-and-1-more` (thirteen `Core\IO\File` and `Core\IO\Metadata` members owing
`rule:testing/feature-proofs`). All nine `Core\IO\File` members are done, with every proof:
`close`, `flush`, `lock`, `read`, `readLine`, `seek`, `tell`, `truncate` and `write`. `read` with
a huge `$max` no longer fills an uncounted Rust buffer: it reads `READ_CHUNK` (64 KB) at a time
and asks `nvs_runtime::affordable` after each chunk, the same as `readLine`. Its attack's last
step reads a 32 MB file under its own `[[app]]` block in the root `nvs.toml` with
`memory = "16M"`. `target/release/nvs.exe` is current with this commit's `io.rs`, and
`docs/perf/members.ndjson` holds fresh records for every `io.rs` member. The four
`Core\IO\Metadata` members are what is left of the goal.

## Next group

**Stage 2: `Core\IO\Metadata` proofs** — one file set: `crates/nvs-stdlib/src/io.rs` (the
`METADATA` rows, its cards and the test module), `docs/examples/core/IO-Metadata/`,
`tests/hostile/core/IO-Metadata/`, `benches/members/core/IO-Metadata/`,
`tests/conformance/core/io-a-stat-is-a-snapshot-and-never-a-live-view-of-the-file.nvst`
(it already calls all four members, so a `// covers:` line is its whole edit).

- [ ] **`Core\IO\Metadata::size`** — owes examples, hostile, perf, tests;
      `rule:testing/feature-proofs`. Member at `crates/nvs-stdlib/src/io.rs:3284`, card at
      `crates/nvs-stdlib/src/io.rs:1910`, every member reads one slot through
      `crates/nvs-stdlib/src/io.rs:3267`. `Core\IO::stat` builds the value
      (`crates/nvs-stdlib/src/io.rs:3239`), so the Rust test drives `stat` first.
- [ ] **`Core\IO\Metadata::modifiedAt`** — same owed set; `crates/nvs-stdlib/src/io.rs:3292`.
      It returns a `Core\Time\Instant`, so its bench does not declare `allocations 0`.
- [ ] **`Core\IO\Metadata::isFile`** — same owed set; `crates/nvs-stdlib/src/io.rs:3299`.
- [ ] **`Core\IO\Metadata::isDir`** — same owed set; `crates/nvs-stdlib/src/io.rs:3306`.

## Backlog

- The four members are one slot read each and cannot fail, so an attack is about the value
  staying a snapshot (the file changes or is deleted after `stat`), not about the member
  breaking; `crates/nvs-stdlib/src/io.rs`'s `metadata_slot` doc owns that reading.
