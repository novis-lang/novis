# Handoff

## State

Goal `core-io-2-2` is under way. `Core\IO::open`, `read`, `readText`, `remove`, `removeDir`,
`size`, `write`, `writeStream`, `stat`, `walk`, `temporaryDir` and `within` have every feature
proof: the description, three examples, an attack, a bench with its figure in
`docs/perf/members.ndjson`, and a Rust `#[test]` with its `covers:` marker in
`crates/nvs-stdlib/src/io.rs`'s test module. One member of the goal is left: `stdin`.

`within` now throws an `IOError` for a base that is missing or is a file. Before, it pinned the
missing base to an existing ancestor and answered. A `.nvst` case pins the fix.

A full `dossier.py --run all` sweep on the rebuilt release binary is green: 1621 examples and 536
hostile cases pass, and the known gaps are unchanged.

## Next group

**Core\IO::stdin, the last member** — one file set: `crates/nvs-stdlib/src/io.rs` (the registry
row, the body and the test module at its end), `tools/dossier.py` and the member's new proof
paths. Every proof program runs with stdin set to `subprocess.DEVNULL`, so `Core\IO::stdin()`
returns `""` in any example, attack or bench as the tools stand. Three examples that all read
nothing do not show the member, so the slice first gives a proof a way to be fed input (for
example a sibling `<name>.in` file, documented in each tree's README), then writes the proofs.
The Rust test cannot read the test process's own stdin, so split the body's read into a function
over `impl std::io::Read` and drive that with a `Cursor`. Run a proof from the repository root.

- [ ] **`Core\IO::stdin`** — owes examples, hostile, perf, tests (`rule:testing/feature-proofs`); proof input via `tools/dossier.py:433`, body at `crates/nvs-stdlib/src/io.rs:2170`. `crates/nvs-stdlib/src/io.rs:385`

## Backlog

- `nvs test`'s `.nvst` runner may already feed a `--STDIN--` section; check `crates/nvs-test`'s module doc before inventing a second spelling for the proof trees.
- `tests/hostile/core/Html/sanitize/01` takes 42s alone against its 60s limit and times out in an 8-way sweep; owner `crates/nvs-stdlib/src/html.rs`.
- `http::socket::tests::socket_ping_keeps_a_quiet_live_peer_open` failed once under verify's parallel load (a 200ms idle window) and passed on the rerun; owner `crates/nvs-stdlib/src/http/socket.rs`.
