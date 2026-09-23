# Handoff

## State

Goal `core-io-2-2` is under way. `Core\IO::open`, `read` and `readText` have every feature proof:
the description, three examples, an attack, a bench with its figure in `docs/perf/members.ndjson`,
and a Rust `#[test]` with its `covers:` marker in `crates/nvs-stdlib/src/io.rs`'s test module.
`read`'s reference card now says the file must be UTF-8 and that a file that is not throws a
`RuntimeError`. Ten members of the goal are left, from `remove` to `writeStream`.

## Next group

**Core\IO filesystem members, one per slice** — one file set: `crates/nvs-stdlib/src/io.rs` (the
registry rows and the test module at its end) plus each member's new proof paths. Take them in
this order:

- [ ] **`Core\IO::remove`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:250`
- [ ] **`Core\IO::removeDir`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:268`
- [ ] **`Core\IO::size`** — owes what `python tools/dossier.py --id 'Core\IO::size'` prints (`rule:testing/feature-proofs`). `crates/nvs-stdlib/src/io.rs:196`

## Backlog

- `Core\IO::open`'s card says a directory opened with `Read` throws an `IOError`. On Linux
  `std::fs::OpenOptions` opens a directory for reading, so only the first read fails. Not checked
  under WSL; `crates/nvs-runtime/src/capability.rs`'s `open` owns it.
- `tests/hostile/core/Html/sanitize/01-tricky-markup-and-a-huge-document.nvs` ran past 60 s under
  `dossier.py --run all` with 8 at a time, once. It was not re-run alone; its owner is `crates/nvs-stdlib/src/html.rs`.
- The rest of the goal after the next group: `stat`, `stdin`, `temporaryDir`, `walk`, `within`,
  `write`, `writeStream`, in that order (`docs/agent/loop-goal.md`).
