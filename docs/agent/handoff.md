# Handoff

## State

**Goal `sqlite-queue`, stage 5 is green on both halves.** The `nvs-cli` check's three names all
resolve and pass; the earliest failing check is now stage 6's matrix leg.

- **The check's third name was a rename, not a test to write.** `crates/nvs-cli/src/worker.rs`
  already carried `the_worker_reads_args_and_script_at_the_positions_the_claim_statement_returns_them`
  over all three claim lists, which is what stage 2's third point ordered ("extend that test rather
  than writing a second one"), so both `docs/agent/loop-goal.toml:7454` and the goal's own
  `.toml` point at it under that name, with a comment saying why. The playbook bullet owns the
  general shape.
- **`open`'s SQL Server refusal is a value now** — `sql_server_gap` at
  `crates/nvs-cli/src/worker.rs:1193` — because a test asserting what an operator is told cannot
  read an `eprintln!` in an arm. The arm itself is unchanged in what it does and still spelled
  rather than left to a `_`.
- **Goal stage 6 item 2 — "queue.rs's own `Dialect` gains its third arm" — is superseded and should
  not be taken**, carried unchanged: `crates/nvs-stdlib/tests/queue.rs` reaches a driver over a
  socket, and this backend's cases are `queue_sqlite.rs`'s, which need no server.
- Nothing is blocked, and no `[context]` field was missing from this session's pack.

## Next group

**Stage 6: the matrix leg, then the cases it makes possible** — item 1 is one edit in
`tools/db-matrix.py`; items 2 and 3 share `tests/conformance/core/` and both read the fixture tree
`examples/queue-sqlite.toml`, which is already on disk. Take item 1 first: it is the check the
driver reports next.

- [ ] **The SQLite leg runs the queue suites** — `SUITES` at `tools/db-matrix.py:114` names only
      `["-p", "nvs-stdlib", "--test", "queue"]`, which is the socket suite; the SQLite backend's
      cases are `crates/nvs-stdlib/tests/queue_sqlite.rs`. The note at `tools/db-matrix.py:108` is
      the home of why that list is narrow, so decide there whether the second target is added for
      every leg or only for the driver with no service (`tools/db-matrix.py:165`) — the four server
      legs would otherwise pay for a suite that asks their server nothing. The check is
      `docs/agent/loop-goal.toml:7473`, under `rule:core-classes/db-one-api`'s *Verification*
      section, which `tools/db-matrix.py:13` cites as what this harness exists to point at.
- [ ] **`tests/conformance/core/queue-push-and-status-run-against-a-sqlite-file.nvst`** — the case
      list is `docs/agent/loop-goal.toml:7487`, the shape to copy is
      `tests/conformance/core/queue-push-refuses-an-enqueue-with-no-queue-configured.nvst:1`, and
      the `[queue]`/`[db.jobs]` pair a case needs to reach a file is `examples/queue-sqlite.toml:1`.
      Under `rule:core-classes/queue-storage-is-a-table`. **First decide how a conformance case
      names that tree at all** — the suite runs `nvs test tests/conformance/` with no `--config`,
      which is what makes this item the one that says whether the two after it are writable.
- [ ] **The other two cases** — `queue-delete-and-purge-answer-the-same-way-on-sqlite.nvst` and
      `queue-cancel-releases-a-dedupe-key-on-sqlite.nvst`, same list at
      `docs/agent/loop-goal.toml:7489`, same rule, same file set.

## Backlog

- Stage 6's fixture checks (`docs/agent/loop-goal.toml:7503` and `:7519`) name
  `examples/queue-sqlite.nvs` and `examples/queue-sqlite.toml`, both on disk; whether they pass was
  not checked this session.
- Goal stage 6 item 4 — a fixture beside `examples/queue.nvs` is its own file, that example's lines
  are frozen (`docs/agent/goals/35-sqlite-queue.md:171`).
- Stage 6 wants no differential case: PHP has no queue
  (`docs/agent/goals/35-sqlite-queue.md:169`).
