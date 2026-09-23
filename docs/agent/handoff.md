# Handoff

## State

Goal `core-io-file-and-1-more` (thirteen `Core\IO\File` and `Core\IO\Metadata` members owing
`rule:testing/feature-proofs`). `close`, `flush` and `lock` are done, with every proof: `about.md`,
three examples, a bench, an attack, a Rust test in `crates/nvs-stdlib/src/io.rs`'s test module
and a `covers:` marker on the existing `.nvst` cases. `Core\IO\File` has its class card and is off
`CLASSES_STILL_OWING_A_CARD`. The root `nvs.toml` grants `fs.read` and `fs.write` to the three
`IO-File` proof trees, so a new `IO-File` proof needs no grant of its own. The test module now has
`handling`, `handle_on`, `on_handle` and `let_go`, which open a `ReadWrite` handle and call any
`Core\IO\File` member on it; the next slices reuse them. No proof found a bug.

## Next group

**Stage 1: `Core\IO\File` proofs** — one file set: `crates/nvs-stdlib/src/io.rs` (the
`FILE` rows and the test module's `on_handle` helper), `docs/examples/core/IO-File/`,
`tests/hostile/core/IO-File/`, `benches/members/core/IO-File/`, `tests/conformance/core/io-*.nvst`.

- [ ] **`Core\IO\File::read`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/io.rs:1366`. `io-an-open-file-is-an-object-and-never-a-resource.nvst`
      is the Novis case to mark with `covers:`.
- [ ] **`Core\IO\File::readLine`** — same owed set; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/io.rs:1375`. `io-readline-*.nvst` are the cases to mark.
- [ ] **`Core\IO\File::write`** — same owed set; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/io.rs:1386`.

## Backlog

- `Core\IO\File::seek`, `tell`, `truncate` (`crates/nvs-stdlib/src/io.rs:1398`), then the four
  `Core\IO\Metadata` members; the goal's list in `docs/agent/loop-goal.md` is the order.
- `nvs agent show` prints no class card text for any class, `Core\IO` included, and
  `tools/reference.py` renders none into `docs/novis.md`; the card reaches `nvs meta --json` only.
  Owned by `rule:core-api/reference-card`; not checked whether that is intended.
