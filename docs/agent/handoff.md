# Handoff

## State

**Goal `m8-db-queue`, stage 8 is complete: recording, applying and the call site all land.**
`push` declares the job's narrowing as five options at the end of its one trailing bag — `grants`
and the four `[limits]` ceilings of `LIMIT_OPTIONS` (`crates/nvs-stdlib/src/queue.rs:1782`) — and
`grants_of`/`limits_of` (`crates/nvs-stdlib/src/queue.rs:2735`) turn what a call site wrote into the
two columns the row already carried, before the connection is reached.

A written `grants:` is checked name by name against `nvs_runtime::capability::granted` asked
unscoped, so a name the enqueuing request does not hold is a `RuntimeError` where it stands and a
name no capability has is a `LogicError`. A written ceiling is set on a clone of the request's own
`nvs_config::Request` and read back by `limits_recorded`, which makes `Request::set` the one reader
that judges it — see the playbook bullet for what that does and does not prove.

`crates/nvs-stdlib/src/queue.rs`'s gap 1 is retired and the two below it renumbered; the goal's
*Not this goal* list now says gaps 1–2. Stage 9 is untouched and all three of its checks are red.

## Next group

**Stage 9: the socket leg, then the CI leg** — one file set: `tools/db-matrix.py`,
`crates/nvs-db/src/matrix.rs`, `.github/workflows/ci.yml` and `tools/ci-changes.py`.
`rule:core-classes/db-unix-socket-path` owns the transport; the stage's three `[[check]]` blocks are
`docs/agent/loop-goal.toml:10720-10748` and none of them passes today.

- [ ] **`tools/db-matrix.py` publishes an `AF_UNIX` endpoint for MySQL, MariaDB and PostgreSQL, and
      runs the TCP case list again over it.** The socket directory is bind-mounted out of each
      container and handed to the leg as `NVS_DB_MATRIX_SOCKET`
      (`crates/nvs-db/src/matrix.rs:73`), whose absence is the whole of the choice between the two
      transports; `tests/handshake.rs` already dials whichever `Location` it is handed. The check
      wants the three `… over a socket: ok` lines after the five TCP ones.
- [ ] **Retire `crates/nvs-db/src/matrix.rs:43` gap 1**, which is that leg being asked for and never
      published — it is `m8-db-queue`-owned and its *Decided:* line is the whole case list again
      over the socket.
- [ ] **`.github/workflows/ci.yml:106` runs `python tools/db-matrix.py --all`, and the `LANES`
      table at `tools/ci-changes.py:34` gains the `db` lane it prints as `db=true`.** CI is not
      running (goal § *Standing decisions*), so both are proven by reading the workflow file and
      the lane tool, never by a remote run.

## Backlog

- Stage 10's rulebook sweep: `core-classes/a-stream-parks-its-read-on-the-connection` still reads
  `designed`, and `docs/agent/loop-goal.toml:10754` is the check.
- `crates/nvs-stdlib/src/queue.rs` gaps 1–2 are `unowned-closures`', not this goal's.
