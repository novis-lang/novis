# Handoff

## State

**Goal `tooling-overhaul` has just started; nothing of it has landed yet.** It replaces the Python
tools with one TypeScript program on Bun, `bun nv`, over typed JSON records under `data/`, and renders
every Markdown file a person reads from them. It also keys every check on what the check reads (side
goal `test-time`), triages the playbook (side goal `playbook-triage`), and turns the generated
feature-proof goals into ordinary goals. Both side goals are retired into it. Goal
`core-json-and-6-more`'s whole list is this goal's Stage 1 floor.

**`main` is frozen until this goal is walked.** No other goal runs beside it, so a stage can move any
file without a conflict. The tag `pre-overhaul` is the rollback point.

The design is the user's, decided on 2026-09-24 question by question. § *The data model* and
§ *Standing decisions* answer every call a session meets. § *What is on disk today, measured* is the
starting point, and every cause there has a `file:line`. Re-check a line number before you edit at
it.

Three habits make this goal cheap. Every session holds to them:

- **A mechanical change goes through a script written for it** under `.agent-tmp/`: the import, a
  rename, a citation rewrite, LF, a batch of triage. Never edit file after file by hand.
- **Never prove a cut with a sweep.** Use `bun nv impact --probe`, `bun nv why`, `bun nv parity` and
  single commands.
- **A Python tool is deleted only after its replacement's parity is green.**

## Next group

**Stage 2: the Rust and Cargo cuts.** One file set: `crates/nvs-cli/src/script.rs`, the four flaky
tests' files, the crates' `Cargo.toml`, the workspace `Cargo.toml`, `tools/verify.py`.

- [ ] **`EDITS` becomes 200** (`crates/nvs-cli/src/script.rs:2513`). The test is renamed
      `units_held_stay_bounded_after_two_hundred_edits`, and its comment is rewritten whole.
- [ ] **`doctest = false`** goes in every crate with no doc-test. The five doc-tests are in
      `nvs-diagnostics` `diagnostic.rs:5` and `lib.rs:7`, `nvs-runtime` `abi.rs:636` and `fmt.rs:25`,
      and `nvs-syntax` `lib.rs:46`.
- [ ] **`RUST_TEST_THREADS`** goes in `verify.py`'s pool, so that pool width times threads is about the
      machine's cores.
- [ ] **The four load-flaky tests wait on a condition**: `nvs-host` `watchdog.rs:926`, `nvs-stdlib`
      `http/transport.rs:4117` and `socket.rs:1241`, `nvs-lsp` `latency.rs:212`.
- [ ] **`[profile.proof]`** goes in the workspace `Cargo.toml`, with its disk cost stated in its
      comment.

## Backlog

Each stage has its own file set, so each normally starts a new session.

- **Stage 3: the foundation.** `package.json`, `tsconfig.json`, `tools/nv/lib/**`, `nv check`,
  `query`, `render` and `selftest`, and `.cache/` in `.gitignore`.
- **Stage 4: the importer.** `tools/nv/import/**`. `--check` must read every legacy home and render
  every generated file identically before Stage 9.
- **Stage 5: the read-only tools**, each proven by `bun nv parity`.
- **Stage 6: one key** (the keystone). Write the probe table first: its red cells are the worklist.
- **Stage 7: proofs.** The dossier's generator and fan-out are deleted, and the rest becomes one pass
  on the proof binary.
- **Stage 8: the writers and the driver**, proven by `parity writers` and `parity orient`.
- **Stage 9: the cutover**, one commit. After it, the next turn is the new driver.
- **Stage 10: the playbook triage**, one section per batch, smallest first.
- **Stage 11: every other tool, CI, the website and the hooks.**
- **Stage 12: no Python, and the contract rewritten.** The run ends at this goal's `GOAL REACHED` on
  purpose. The user then starts `bun nv loop`, and the chain goes on with goal `core-math-1-3`.
