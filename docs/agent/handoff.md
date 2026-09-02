# Handoff

## State

**§ 7's options bag is complete: all three options now reach the connection.** `retries` is read at
`crates/nvs-stdlib/src/db.rs:3325` out of slot 4 (`RETRIES_ARG`) and the helper's body is a loop —
each attempt gets its own `BEGIN` and its own scope object, and it re-runs while four conditions
hold together: an attempt is left, the transaction is outermost (`nvs_db::PgConn::depth`, new at
`crates/nvs-db/src/pg.rs:708`), nothing called `rollBack`, and the failure the commit answered with
carries a `nvs_db::DbErrorKind` that `is_retryable`.

**Two deliberate narrowings of § 7, and known gap 9 at `crates/nvs-stdlib/src/db.rs:126` is both.**
There is **no wait between attempts** — § 7's backoff suspends the coroutine, `nvs-runtime` has no
yielder, and the gap argues why `nvs_host::blocking` is the worse of the two available answers
(correlated conflicts drain the pool every other request needs for real blocking work). And **only
the commit's conflict is retried**: a deadlock raised by a statement *inside* the closure arrives as
a `Fault` with no kind on it, which is this module's gap 4. One change closes both.

**`nvs-db`'s `commit` now zeroes the depth when an *outermost* `COMMIT` is refused by the server**
(`crates/nvs-db/src/pg.rs:3072`). PostgreSQL has already rolled the transaction back by then, so the
count follows the connection; a refused `RELEASE SAVEPOINT` moves nothing, and neither does a
refusal this crate made itself — § 4's busy connection never sent the command, which
`nvs_db::ServerError::of` is the test for. All three sides are pinned in that module's tests.
Without this a retry would have sent `SAVEPOINT` against nothing.

**The `args` wall is unchanged and is still the user's call**: Stage 5's
`args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) cannot see the two
`nvs-stdlib` tests, so five of its seven names stay unfound there.

**The driver's acceptance line still names `examples/queue.nvs`** — Stage 8's unlanded `Core\Queue`
(ADR 0084), not a regression. **The CA is still not in git**; `nvs_host::tls`'s module doc owns why.

**`orient.py` still did not print ADR 0067 § 7**, which the previous handoff already asked for: add
`0067 § 7` to `[context] adrs` in `docs/agent/loop-goal.toml`. Every slice of this group was
specified by it and it was sliced by hand again.

## Next group

**Give `Db\DbError` § 8's kind — the one change that closes gap 4 and the rest of gap 9. The file set
is `crates/nvs-hir/src/errors.rs`, `crates/nvs-runtime/src/throwable.rs` and
`crates/nvs-stdlib/src/db.rs`.**

- [ ] **`Db\DbError` joins spec § 10's tree** — a row in `crates/nvs-hir/src/errors.rs:72`'s `TREE`
      under `RuntimeError`, and the variant beside `DbRolledBack` at
      `crates/nvs-runtime/src/throwable.rs:146` in the enum at
      `crates/nvs-runtime/src/throwable.rs:93`. The playbook's *Running things* bullet on adding a
      `TREE` row names the test in `nvs-ir` that fails and why its message does not say so. ADR 0067
      § 8.
- [ ] **`statement_failure` throws it** — `crates/nvs-stdlib/src/db.rs:2495` currently answers
      `Fault::thrown`, so every server refusal is a bare `RuntimeError`; it holds the
      `nvs_db::ServerError` (kind, `sqlState`, constraint) and drops all of it. Decide there whether
      the kind rides on the throw or on a slot a program can read. ADR 0067 § 8.
- [ ] **The retry loop reads the closure's conflict too** — `crates/nvs-stdlib/src/db.rs:3325`'s
      `Err(fault)` arm currently rolls back and re-raises; with a kind reachable off the fault it
      becomes the same four-condition test the commit path already runs, and gap 9 at
      `crates/nvs-stdlib/src/db.rs:126` loses its second half. ADR 0067 § 7.

## Backlog

- `examples/transaction.nvs` writes no `{isolation: …}` and no `{retries: n}`, so nothing exercises
  either end to end — ADR 0067 § 7.
- Widening Stage 5's check `args` is one edit that closes Stage 4's and Stage 5's together —
  `docs/agent/loop-goal.toml:2830`.
- Stage 8's `Core\Queue` is what the driver's acceptance check names — ADR 0084.
- `open` waits on a registry type for a shape *parameter* — `crates/nvs-stdlib/src/db.rs` gap 1.
- `stream`/`streamAs` are owed on both `Queryable` classes — that module's gap 5.
- § 7's backoff needs `nvs-runtime`'s gap 3, the yielder — `crates/nvs-runtime/src/lib.rs:208`.
