# Handoff

## State

Goal `core-io-file-and-1-more` (thirteen `Core\IO\File` and `Core\IO\Metadata` members owing
`rule:testing/feature-proofs`). `close`, `flush`, `lock`, `read`, `readLine` and `write` are done,
with every proof. The `readLine` proofs found two bugs, and both are fixed. A line with no end
grew an uncounted buffer to the size of the file before the memory limit fired; each 8 KB chunk
now asks `nvs_runtime::affordable` first. A line that is not UTF-8 moved the handle past it; now
the handle stays at the line's start, the same promise `read` makes. The helper's `# Decision`
and the card say so. The readLine attack runs under its own `[[app]]` entry block in the root
`nvs.toml` with `memory = "16M"`. `target/release/nvs.exe` is current with this commit's `io.rs`,
and `docs/perf/members.ndjson` holds fresh records for every `io.rs` member.

## Next group

**Stage 1: `Core\IO\File` proofs** — one file set: `crates/nvs-stdlib/src/io.rs` (the
`FILE` rows and the test module's `read_on`/`read_line_on`/`write_on`/`on_handle` helpers),
`docs/examples/core/IO-File/`, `tests/hostile/core/IO-File/`, `benches/members/core/IO-File/`,
`tests/conformance/core/io-*.nvst`.

- [ ] **`Core\IO\File::read` with a huge `$max` on a huge file** — `rule:programs/memory-priority`.
      `crates/nvs-stdlib/src/io.rs:2352` reads with `take(max).read_to_end`, which fills a Rust
      buffer the request's balance does not see. Read in chunks and ask `nvs_runtime::affordable`
      per chunk, the way `readLine` now does (`crates/nvs-stdlib/src/io.rs:2456`). Add a step to
      `tests/hostile/core/IO-File/read/` and a position assertion to the Rust test.
- [ ] **`Core\IO\File::seek`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/io.rs:2550`.
- [ ] **`Core\IO\File::tell`** — same owed set; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/io.rs:2584`.
- [ ] **`Core\IO\File::truncate`** — same owed set; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/io.rs:2632`.

## Backlog

- `Core\IO\Metadata`'s `size`, `modifiedAt`, `isFile` and `isDir` owe every proof and their help
  (`python tools/dossier.py --owed --group 'Core\IO\Metadata'`), after the `File` group.
