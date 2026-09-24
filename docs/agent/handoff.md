# Handoff

## State

**Goal `tooling-overhaul`, Stage 2 (the Rust and Cargo cuts) is four fifths landed.** The goal
replaces the Python tools with one TypeScript program on Bun, `bun nv`, over typed JSON records
under `data/` (loop-goal.md § *The data model*). `main` is frozen until it is walked, and the tag
`pre-overhaul` is the rollback point.

On disk from Stage 2: the bounded-units test runs 200 edits (`nvs-cli bin nvs` went from 211 s to
under 3 s alone); `doctest = false` in every library crate but `nvs-diagnostics`, `nvs-runtime` and
`nvs-syntax`, which hold the five doc-tests; `[profile.proof]` in the workspace `Cargo.toml`, which
Stage 7 wires in; `RUST_TEST_THREADS` in `tools/verify.py`'s pool.

**`RUST_TEST_THREADS` is weighted, not flat, and that is my call against the goal's wording** ("pool
width times threads is about the machine's cores"). Measured alone: `live_config` takes 14 s at 16
threads, 48 s at 4 and 147 s at 1, so one thread per binary would make the slowest binary the whole
step. Each binary gets threads in proportion to its last time against the slowest one's; the short
ones get one each. The reasoning is `tools/verify.py` § *Why `test` runs its binaries side by side*.
If the user wants the flat cap, it is the one loop in `run_tests`.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its replacement's
parity is green.

## Next group

**Stage 2: the Rust and Cargo cuts, its last item.** One file set: the four load-flaky tests' files.
Each waits on a condition, never on a wall-clock bound (loop-goal.md § *Stage 2*). Re-check each
line number first; after these, Stage 3 (`bun nv`, its library and its index) starts.

- [ ] **`nvs-host` watchdog test waits on a condition** (`crates/nvs-host/src/watchdog.rs:926`).
- [ ] **`nvs-stdlib` HTTP transport test waits on a condition**
      (`crates/nvs-stdlib/src/http/transport.rs:4117`).
- [ ] **`nvs-stdlib` socket test waits on a condition** (`crates/nvs-stdlib/src/socket.rs:1241`).
- [ ] **`nvs-lsp` latency test waits on a condition** (`crates/nvs-lsp/tests/latency.rs:212`).

## Backlog

- Stage 3 onward: loop-goal.md § *Stage 3* and after.
- A crate that gains a runnable doc example must delete its `[lib] doctest = false` table; nothing
  checks that yet. A `bun nv` check could, from Stage 3 on (loop-goal.md § *Stage 2*).
