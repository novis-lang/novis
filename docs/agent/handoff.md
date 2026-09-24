# Handoff

## State

**Goal `tooling-overhaul`: Stage 2 (the Rust and Cargo cuts) is landed whole; Stage 3 starts
next.** The goal replaces the Python tools with one TypeScript program on Bun, `bun nv`, over
typed JSON records under `data/` (loop-goal.md § *The data model*). `main` is frozen until it is
walked, and the tag `pre-overhaul` is the rollback point. Bun 1.3.5 is on this machine's PATH;
`package.json`, `tsconfig.json` and `tools/nv/` do not exist yet, which is why the driver's
`bun nv selftest` check reads `Script not found "nv"`. That is an item still open. It is not a
regression.

On disk from Stage 2: 200-edit bounded-units test, `doctest = false`, `[profile.proof]` (Stage 7
wires it in), weighted `RUST_TEST_THREADS` in `tools/verify.py` (my call against the goal's
"flat" wording; the reasoning is in `verify.py` § *Why `test` runs its binaries side by side*). The
four load-flaky tests now wait on a condition. The watchdog's "no report" half is judged against
the lateness the worker measured itself, and its wedged worker stays wedged until the report
arrives. The retry test checks that the call came back before its own deadline. The socket ping
test's peer stays quiet until it has answered a set number of pings (`Say::Answering`). The LSP
latency guards keep sampling until the minimum is under the ceiling, up to `MAX_RUNS`. The goal
named the socket test as `socket.rs:1241`; it is `crates/nvs-stdlib/src/http/socket.rs`.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green.

## Next group

**Stage 3: the foundation** — one file set: `package.json`, `bun.lock`, `tsconfig.json`,
`tools/nv/**`, `.gitignore`, `tools/verify.py` (one step). loop-goal.md § *Stage 3* is the spec.

- [ ] **The package**: root `package.json` with the one `nv` script and Bun pinned in `engines`,
      dev deps `typescript` + `@types/bun`, runtime dep `smol-toml` only, strict `tsconfig.json`,
      `node_modules/` in `.gitignore` (`docs/agent/loop-goal.md:262`).
- [ ] **`tools/nv/lib/`**: `schema`, `store`, `index` (SQLite with foreign keys), `prose`, `git`,
      `paths`, `proc`, `render`, each with its `bun test` (`docs/agent/loop-goal.md:269`).
- [ ] **Commands `check`, `query`, `render`, `selftest`**. `selftest` prints the two lines the
      acceptance check wants: `nv selftest: the types check` and `nv selftest: every test passes`
      (`docs/agent/loop-goal.md:276`).
- [ ] **`verify.py` gains an `nv selftest` step** beside the cargo steps (`tools/verify.py:1023`).

## Backlog

- Stage 4, the importer, follows Stage 3 (loop-goal.md § *Stage 4*).
