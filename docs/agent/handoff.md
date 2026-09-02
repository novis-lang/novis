# Handoff

## State

**Three of ADR 0067 § 8's five properties are declared, typed and slotted, and two of them are
written on a real PostgreSQL refusal.** `constraint` landed this session as
`nvs_runtime::CONSTRAINT_SLOT` (`crates/nvs-runtime/src/throwable.rs:122`) and its compiler copy
(`crates/nvs-hir/src/errors.rs:215`), both `KIND_SLOT + 3`, appended to `KIND`
(`crates/nvs-hir/src/errors.rs:157`) in § 8's own order so nothing before it moved.
`statement_failure` (`crates/nvs-stdlib/src/db.rs:2632`) pushes it onto the boxed slice only where
`nvs_db::ServerError::constraint` is `Some` — most of § 8's eleven kinds name no constraint, and an
unwritten slot already reads `null`, which is why the type is `?string` in
`nvs_types::error_lib::own_properties` (`crates/nvs-types/src/error_lib.rs:149`).

**`examples/db.nvs`'s `catch` is now the fixture for all three.** It echoes `kind`, then
`$refused->sqlState`, then `$refused->constraint`, each with its `?? "none"` spelled out, and the
`kind = "exact"` check pins `kind=UniqueViolation`, `sqlState=23505`, `constraint=people_pkey`
against `tests/db/compose.yaml`'s PostgreSQL. Both copies of the `want` list carry the two new lines
— `docs/agent/loop-goal.toml:2970` and `docs/agent/goals/5-database.toml:2970` — because
`goal-switch.py` restores the live file from the second.

**`sql` is the one property still absent, and it is not the same shape of work as the other three.**
The three landed ones each needed a seeded type and a slot; `sql` needs those *plus* a signature
change, because `statement_failure` is handed `named` and the config block and never the statement
text. There are **twelve** call sites (`crates/nvs-stdlib/src/db.rs:2883` through `:3703`), and the
transaction ones pass a member name rather than any SQL at all — so what a `BEGIN`/`COMMIT` refusal
puts in `sql` is a decision that slice makes. § 8 permits the text (a `Throwable` message is a
`secret` sink, but the SQL is developer-authored); no bound value may join it.

**`driverCode` is declared and still written nowhere**, and on PostgreSQL never will be —
`nvs_db::ServerError`'s doc owns why. The slot exists ahead of MySQL's driver because § 8 fixes the
property order.

**Unchanged and still true.** The driver's acceptance line names `examples/queue.nvs` — Stage 8's
unlanded `Core\Queue` (ADR 0084), not a regression; its `[[check]]` is
`docs/agent/loop-goal.toml:2927`. Stage 5's `args = ["test", "-p", "nvs-db"]`
(`docs/agent/loop-goal.toml:2830`) still cannot see the two `nvs-stdlib` tests, and is still the
user's call. The CA is still not in git; `nvs_host::tls`'s module doc owns why.

**`orient.py` printed ADR 0067 §§ 1, 9 and 13 — not § 7 or § 8, which specify this group and the
last four.** Seven handoffs have now asked: add `0067 § 7` and `0067 § 8` to `[context] adrs` in
`docs/agent/loop-goal.toml`.

## Next group

**Land § 8's last property, `sql`. The file set is the one this session just closed:
`crates/nvs-hir/src/errors.rs`, `crates/nvs-runtime/src/throwable.rs`,
`crates/nvs-types/src/error_lib.rs`, `crates/nvs-stdlib/src/db.rs`,
`crates/nvs-codegen/tests/throwing.rs`, `examples/db.nvs` and the two goal tomls.**

- [ ] **`statement_failure` is handed the statement text** — add the parameter at
      `crates/nvs-stdlib/src/db.rs:2632` and fill it at all twelve call sites between
      `crates/nvs-stdlib/src/db.rs:2883` and `crates/nvs-stdlib/src/db.rs:3703`; the four transaction
      and batch ones (`:3479`, `:3603`, `:3703`) have no single statement to name, so decide there
      whether they pass `None` or the `BEGIN`/`COMMIT` they issued. ADR 0067 § 8.
- [ ] **`sql` becomes slot 4** — append to `KIND` at `crates/nvs-hir/src/errors.rs:157`, add the
      constant beside `CONSTRAINT_SLOT` at `crates/nvs-hir/src/errors.rs:215` and
      `crates/nvs-runtime/src/throwable.rs:122`, export it at `crates/nvs-runtime/src/lib.rs:351`,
      seed `?string` at `crates/nvs-types/src/error_lib.rs:149`, and add the two assertions at
      `crates/nvs-codegen/tests/throwing.rs:98` and `crates/nvs-codegen/tests/throwing.rs:113`.
      ADR 0067 § 8.
- [ ] **A leg pinning `sql`** — extend the `catch` at `examples/db.nvs:159` with a line echoing
      `$refused->sql`, and add it to the `want` list at `docs/agent/loop-goal.toml:2970` **and** the
      copy at `docs/agent/goals/5-database.toml:2970`. ADR 0067 § 8.

## Backlog

- § 13's pool is Stage 5 to 7 of the goal, per `docs/agent/loop-goal.toml`.
- Stage 5's `-p nvs-db` check cannot see the two `nvs-stdlib` tests it names — the user's call.
- `docs/agent/loop-goal.toml`'s `[context] adrs` is missing `0067 § 7` and `§ 8`.
- `Core\Queue` (ADR 0084) is Stage 8 and holds the acceptance run at four checks an iteration.
- The TLS CA under `tests/db/` is not in git; `nvs_host::tls`'s module doc owns why.
