# Handoff

## State

**Goal `sqlite-queue`, stage 6: the fixture is landed with the acceptance data that runs it.**
`examples/queue-sqlite.nvs` is the first queue fixture in this repository that needs no server —
0.7s, five frozen lines, five consecutive clean runs natively. The goal's one ADR slot stays taken
by [0170](../decisions/0170.md).

- **The fixture runs under its own tree**, `examples/queue-sqlite.toml`, named with `nvs run
  --config` exactly as `examples/cache-shared-socket.nvs` names its own. `[queue]` is one block per
  deployment, so a second queue cannot be spelled beside the repository's; that file carries the
  `[[app]]` and `script.spawn` grant a job needs, because `--config` disables the search for
  `./nvs.toml` entirely.
- **Its database is `tests/db/queue-sqlite.db`** (gitignored, resolved against the config file per
  `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`), converged by a
  `command` check running `nvs queue migrate --config examples/queue-sqlite.toml`. That ordering is
  structural rather than positional: every `command` check runs before every non-floor program
  check (`tools/loop.py:2664`), and an ordinary `command` check is not remembered across runs, so a
  deleted file is re-created by the next sweep.
- **The fixture is in `[valgrind] skip`.** The sweep runs `nvs run <file>` with no arguments, so
  without the flag this program is the PostgreSQL queue under a SQLite fixture's name. The cost is
  that no valgrind leg sweeps the SQLite queue paths at all; it is in the backlog rather than
  worked around here.
- **Not verified on the WSL leg.** The Linux CLI under `/var/tmp/nvs-target-wsl` predates stage 5
  and rebuilding it would have taken the running driver's target lock for minutes. What is
  unchecked is SQLite's file locking on the drvfs mount, which is where this fixture would fail
  first on that leg.
- The two goal toml copies now agree in content again; nothing is blocked and no `[context]` field
  was missing from this session's pack.

## Next group

**Stage 6: the suites on the SQLite leg** — one file set: `crates/nvs-stdlib/tests/queue.rs` and
`tools/db-matrix.py`, in this order, because the leg cannot run a suite whose dialect table has no
SQLite arm.

- [ ] **`crates/nvs-stdlib/tests/queue.rs`'s own `Dialect` gains its third arm** —
      `crates/nvs-stdlib/tests/queue.rs:313` is the two-armed enum, beside the worker's that
      already has three, under `rule:concurrency/claiming-is-one-statement`. The `Once` per schema
      at `crates/nvs-stdlib/tests/queue.rs:397` is the trap the goal names: a poisoned one reports
      nothing at all.
- [ ] **`tools/db-matrix.py`'s SQLite leg runs the queue suites** — `tools/db-matrix.py:165` is the
      leg ("a scratch file, no container") and its suite list is what this extends, under
      `rule:core-classes/db-drivers-are-an-enum`. The driver roster is not edited; the check that
      goes green is `the SQLite leg runs the queue suites, with no container at all`.

## Backlog

- The three conformance cases of stage 6 — `docs/agent/loop-goal.toml`, stage `6 the cases`. How a
  `.nvst` case reaches a *configured* SQLite queue is not checked, and it is the first question
  that slice has to answer.
- The sentence an operator reads beside `[queue] workers`, saying where a second worker stops
  buying anything on SQLite — the goal's own § *Standing decisions* owes it.
- No valgrind leg sweeps the SQLite queue paths, because the sweep passes no `--config` —
  `docs/agent/loop-goal.toml`'s `[valgrind] skip` comment is where the reason lives.
- `examples/queue-sqlite.nvs` runs three jobs against one file per sweep and nothing empties
  `tests/db/queue-sqlite.db`; the queue names are drawn per run, so the counters stay right and the
  file only grows.
