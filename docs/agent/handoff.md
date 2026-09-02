# Handoff

## State

**ADR 0067 § 8 now records what a wider throw costs, and the number is measured rather than
argued.** `nvs_runtime::Fault` is the error half of every helper's `Result`, so its width is paid on
the *successful* call too: one inline `(slot, value)` pair is **56 B**, five pairs inline **152 B**,
and one `Box<[(usize, Value)]>` **48 B**. The boxed slice is the decision — narrower than the single
pair it replaced, because a `Box<[_]>` is two words where the pair is three — and it is landed:
`Fault::ThrownWithSlots` (`crates/nvs-runtime/src/abi.rs:108`) is the variant,
`Ctx::raise_with_slots` (`crates/nvs-runtime/src/ctx.rs:3400`) its one destination, and
`Thrown::new_as` takes `&[(usize, Value)]` and releases each value whose slot the class is too narrow
to hold. `thrown_with_slot` is unchanged for a one-slot caller, so no throwing site moved.

**§ 8's `sqlState` and `driverCode` are slots 1 and 2 on `Core\Db\DbError`.** They are appended to
`KIND` (`crates/nvs-hir/src/errors.rs:155`) in § 8's own order, so `kind` keeps slot 0 as the
remaining two land; both are `?T` in `nvs_types::error_lib`, which is why nothing seeds them — an
unwritten slot already reads `null`, unlike `ParseError`'s non-nullable `issues`.
`statement_failure` (`crates/nvs-stdlib/src/db.rs:2634`) fills `sqlState` from
`nvs_db::ServerError`'s own field wherever a server worded the refusal, and falls back to the
one-slot `kind`-only throw where nothing did. **`driverCode` is declared and never written**, and on
PostgreSQL never will be: the `SQLSTATE` is that server's only code, which `nvs_db::ServerError`'s
doc owns. The slot exists ahead of MySQL's driver because § 8 fixes the property order.

**`constraint` and `sql` are the two still absent**, each owing a seeded type in
`nvs_types::error_lib::own_properties` before its row may join `KIND`. `constraint` has a live source
(`nvs_db::ServerError::constraint`); `sql` does not — `statement_failure` is handed `named` and the
config block, never the statement text, so that one is a signature change at its call sites first.

**Unchanged and still true.** The driver's acceptance line names `examples/queue.nvs` — Stage 8's
unlanded `Core\Queue` (ADR 0084), not a regression; its `[[check]]` is
`docs/agent/loop-goal.toml:2927`. Stage 5's `args = ["test", "-p", "nvs-db"]`
(`docs/agent/loop-goal.toml:2830`) still cannot see the two `nvs-stdlib` tests, and is still the
user's call. The CA is still not in git; `nvs_host::tls`'s module doc owns why.

**`orient.py` printed ADR 0067 §§ 1, 9 and 13 — not § 7 or § 8, which specify this group and the
last three.** Six handoffs have now asked: add `0067 § 7` and `0067 § 8` to `[context] adrs` in
`docs/agent/loop-goal.toml`.

## Next group

**Pin the raw pair in a program, then land the third value. The file set is `examples/db.nvs`,
`docs/agent/loop-goal.toml`, `docs/agent/goals/5-database.toml`, `crates/nvs-hir/src/errors.rs`,
`crates/nvs-types/src/error_lib.rs` and `crates/nvs-stdlib/src/db.rs`.**

- [ ] **A leg pinning `sqlState` beside `kind`** — extend the `catch` at `examples/db.nvs:159` to
      echo `$refused->sqlState` on its own line (`23505` against `tests/db/compose.yaml`'s
      PostgreSQL), and add that line to the `want` list at `docs/agent/loop-goal.toml:2970` **and**
      the copy at `docs/agent/goals/5-database.toml:2970` — the second is not optional, because
      `goal-switch.py` restores the live file from it. ADR 0067 § 8.
- [ ] **`constraint` becomes slot 3** — append to `KIND` at `crates/nvs-hir/src/errors.rs:155`, add
      a `?string` arm beside `sqlState`'s at `crates/nvs-types/src/error_lib.rs:124` (a row without
      one `panic!`s at seed time), declare `CONSTRAINT_SLOT` beside
      `crates/nvs-hir/src/errors.rs:188` and its runtime copy at
      `crates/nvs-runtime/src/throwable.rs:85`, and fill it from `ServerError::constraint` in the
      `Some(server)` arm at `crates/nvs-stdlib/src/db.rs:2634`. ADR 0067 § 8.
- [ ] **A leg pinning `constraint`** — the same `catch` at `examples/db.nvs:159` and the same two
      `want` lists (`docs/agent/loop-goal.toml:2970`, `docs/agent/goals/5-database.toml:2970`), once
      the slice above lands; PostgreSQL names `people_pkey`. ADR 0067 § 8.

## Backlog

- `sql` is § 8's last raw value and needs the statement text carried into `statement_failure` first —
  `crates/nvs-stdlib/src/db.rs:2628`. ADR 0067 § 8.
- Stage 5's `-p nvs-db` check cannot see its two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`.
- `Core\Queue` (ADR 0084) is Stage 8 and unlanded; it is what the acceptance line reports.
- `open` waits on a shape-parameter type — `docs/implementation-plan.md`'s *Open now*.
- The TLS CA is not in git — `crates/nvs-host/src/tls.rs`'s module doc.
