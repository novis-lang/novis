# Handoff

## State

**ADR 0067 § 8 is complete on PostgreSQL: all five properties are declared, typed and slotted, and
four of the five are written on a real refusal.** `sql` landed this session as
`nvs_runtime::SQL_SLOT` (`crates/nvs-runtime/src/throwable.rs:124`) and its compiler copy
(`crates/nvs-hir/src/errors.rs:217`), both `KIND_SLOT + 4`, appended to `KIND`
(`crates/nvs-hir/src/errors.rs:157`) so nothing before it moved. `driverCode` is the one property
still written nowhere, and on PostgreSQL never will be — `nvs_db::ServerError`'s doc owns why.

**`sql` carries the statement the caller wrote, not `Statement::sql`'s rewrite of it.** That is the
decision the slice had to make and `statement_failure`'s doc comment
(`crates/nvs-stdlib/src/db.rs:2632`) is its home: the rewritten form spells its markers the way one
driver wants them (`$1` here, `?` on MySQL), so carrying it would make one property of a
deliberately normalised error read differently per driver. So the members read § 18's `$sql`
argument a second time — `let source = args[1].as_text()` in `queried_rows` and in `execute` — and
`executeMany` does it inline.

**§ 7's transaction control statements pass `None`.** A `BEGIN`, a `COMMIT` or a `SAVEPOINT` is this
runtime's own text and no program asked for it by name; `?string` already has an absent case that
costs no slot. That is both transaction call sites.

**There were eleven `statement_failure` call sites, not the twelve the last handoff predicted.**
`crates/nvs-stdlib/src/db.rs:2883` through `:3746`.

**`examples/db.nvs`'s `catch` is the fixture for four of the five.** It echoes `kind`, `sqlState`,
`constraint` and now `sql`, each with its `?? "none"` spelled out, and the `kind = "exact"` check
pins `sql=insert into people (id, name) values (?, ?)` — the `?` markers, not the `$1` the driver
sent — against `tests/db/compose.yaml`'s PostgreSQL. Both copies of the `want` list carry the new
line, `docs/agent/loop-goal.toml:2999` and `docs/agent/goals/5-database.toml:2999`, because
`goal-switch.py` restores the live file from the second.

**Stage 5's own remaining half is blocked, not skipped.** § 7's backoff is this module's known gap 9
(`crates/nvs-stdlib/src/db.rs:139`) and it waits on `nvs-runtime`'s known gap 3 — a helper cannot
suspend the coroutine yet — so the two ways to wait without a yielder are both worse than no wait.
Do not re-derive that; the gap's own text argues it.

**Unchanged and still true.** The driver's acceptance line names `examples/queue.nvs` — Stage 8's
unlanded `Core\Queue` (ADR 0084) in full, not a regression; its `[[check]]` is
`docs/agent/loop-goal.toml:2927`, and no member of that class exists in `nvs-stdlib` at all. Stage
5's `args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) still cannot see the two
`nvs-stdlib` tests, and is still the user's call. The CA is still not in git; `nvs_host::tls`'s
module doc owns why.

**`orient.py` printed ADR 0067 §§ 1, 9 and 13 — not § 7 or § 8.** Eight handoffs have now asked: add
`0067 § 7` and `0067 § 8` to `[context] adrs` in `docs/agent/loop-goal.toml`. § 13 being in the pack
is what makes the next group cheap, so the manifest is right about that one.

## Next group

**Stage 7's pool — ADR 0067 § 13, which `orient.py` already prints in full. The file set is
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/db.rs` and
`crates/nvs-db/src/pg.rs`.**

- [ ] **`[db.<name>.pool]`'s four bounds are read from config** — § 13's `max`, `idle`, `lifetime`
      and `acquire`, plus the `pool = false` switch, beside `statement_cache` at
      `crates/nvs-config/src/tree.rs:552`, resolved in `crates/nvs-config/src/db.rs`. Finite with
      nothing configured, per ADR 0074. ADR 0067 § 13.
- [ ] **The per-core store, keyed as § 2 already keys** — a request's connection is released to a
      pool owned by the core rather than closed, at
      `crates/nvs-runtime/src/ctx.rs:3001`'s `hold_open_connection`, which is where known gap 3 of
      `crates/nvs-stdlib/src/db.rs:81` says this belongs. No lock on the acquire path. ADR 0067 § 13.
- [ ] **The reset is the gate, and a failed reset destroys the connection** — PostgreSQL's targeted
      reset is already `crates/nvs-db/src/pg.rs:2958`'s `reset_session`; what is missing is that
      nothing may rejoin the pool without it having returned `Ok`. ADR 0067 § 13.

## Backlog

- Stage 6's other four drivers, MariaDB its own — `docs/agent/loop-goal.md` § *Stage 6*.
- Stage 8's `Core\Queue`, ADR 0084 whole — `docs/agent/loop-goal.md` § *Stage 8*.
- § 7's retry backoff, gated on `nvs-runtime`'s gap 3 — `crates/nvs-stdlib/src/db.rs:139`.
- `open` waits on a shape-parameter type — `crates/nvs-stdlib/src/db.rs:66`.
- `{retries: n}` has no fixture inducing a real conflict — `docs/agent/loop-goal.md` § *Stage 5* 15.
- Stage 5's `-p nvs-db` check cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`.
