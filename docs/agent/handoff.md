# Handoff

## State

**Goal `sqlite-queue`: every stage-6 check passes on this machine**, and stage 6 is the last stage in
`docs/agent/loop-goal.toml`. `python tools/verify.py` is 9 of 9 green (conformance 1720).

- The matrix leg prints `sqlite: ok`, the three conformance cases pass, and both fixture checks do —
  `nvs queue migrate --config examples/queue-sqlite.toml` reports both tables applied and
  `examples/queue-sqlite.nvs` prints its five frozen lines.
- **The three cases each carry the queue's two `create table` statements**, because a `.nvst` reaches
  no operator command and the schema has no `Core` spelling. The one home is still
  `nvs_stdlib::queue::schema()`; the copies come from `nvs queue migrate --dry-run` and break loudly
  rather than drifting. That is the goal's one open question and it is in
  [carried-gaps.md](carried-gaps.md) § *Unowned*, so it survives the switch.
- **The goal's `workers` obligation is landed**: `rule:concurrency/who-runs-a-job-is-configuration`
  now says where the count stops buying throughput on a single-writer file, which is where an
  operator reads the key.
- Goal stage 6 item 2 — "queue.rs's own `Dialect` gains its third arm" — stays superseded: that suite
  reaches a driver over a socket, and this backend's cases are `queue_sqlite.rs`'s.
- Nothing is blocked, and no `[context]` field was missing from this session's pack.

## Next group

**Whatever the driver's own acceptance names, and otherwise the next goal's** — if this goal is
re-opened, one file set: `tests/conformance/core/queue-*-on-sqlite*.nvst` and
`crates/nvs-stdlib/src/queue.rs`.

- [ ] **Decide whether the queue's schema gets a `Core` spelling** — `crates/nvs-stdlib/src/queue.rs:313`
      is the value, and the three cases that copy it start at
      `tests/conformance/core/queue-push-and-status-run-against-a-sqlite-file.nvst:22`. Under
      `rule:core-classes/queue-storage-is-a-table`, whose one-home argument is what the copies sit
      against. A new `Core` member is surface, so this is the user's call rather than a session's.
- [ ] **Check whether SQLite's dead-letter path has a case at all** — the schema half is pinned, and
      whether a job that exhausts its attempts is asserted on this backend was not checked:
      `crates/nvs-stdlib/tests/queue_sqlite.rs:1` is where such a case belongs, under
      `rule:concurrency/claiming-is-one-statement`'s visibility-timeout paragraph.

## Backlog

- The DDL copy in the three cases — indexed in [carried-gaps.md](carried-gaps.md) § *Unowned*.
- The four server legs of `tools/db-matrix.py` deliberately do not run `queue_sqlite`;
  `tools/db-matrix.py:119` owns why.
