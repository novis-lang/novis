# Handoff

## State

Goal `core-io-file-and-1-more` (thirteen `Core\IO\File` and `Core\IO\Metadata` members owing
`rule:testing/feature-proofs`). `close`, `flush`, `lock` and `read` are done, with every proof.
The `read` proofs found a bug, and it is fixed: a `$max` inside a multi-byte character threw and
skipped the bytes it had read. Now `read` ends on a whole character, gives the cut bytes back,
and a read that throws does not move the handle. The helper's `# Decision` says so, and
`tests/conformance/core/io-read-ends-on-a-whole-character-and-a-refused-read-does-not-move.nvst`
pins it. The test module has `handling`, `handle_on`, `on_handle`, `read_on` and `let_go` for any
`Core\IO\File` member. The root `nvs.toml` grants the three `IO-File` proof trees `fs.read` and
`fs.write`. `target/release/nvs.exe` is current with this commit's `io.rs`.

## Next group

**Stage 1: `Core\IO\File` proofs** — one file set: `crates/nvs-stdlib/src/io.rs` (the
`FILE` rows and the test module's `read_on`/`on_handle` helpers), `docs/examples/core/IO-File/`,
`tests/hostile/core/IO-File/`, `benches/members/core/IO-File/`, `tests/conformance/core/io-*.nvst`.

- [ ] **`Core\IO\File::readLine`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/io.rs:1375`. `readLine` returns `?string`, so its Rust test needs a
      helper like `read_on` that also reads `null`; a line longer than `LINE_CHUNK` (8 KB) and a
      `\r` at the end of a chunk are the edges to attack.
- [ ] **`Core\IO\File::write`** — same owed set; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/io.rs:1378`. `io-an-open-file-is-an-object-and-never-a-resource.nvst`
      already calls it and needs only a `covers:` marker.

## Backlog

- `Core\IO\File::read`'s `$max` parameter text says only "fewer when the file ends first"; the
  returns text covers the cut character. Reword it with the next `io.rs` edit, which re-measures
  the perf anyway (`crates/nvs-stdlib/src/io.rs`, `FILE_READ_DOC`).
