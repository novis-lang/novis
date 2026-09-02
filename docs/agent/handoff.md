# Handoff

## State

**Stage 5 is landed as far as `-p nvs-db` can host it**, both cases in `crates/nvs-db/src/pg.rs`:
`a_db_error_message_contains_no_bound_value` (§ 8's rule that a bound value reaches the wire and
none of the sentence, the `ServerError` or the `Debug` a trace prints — swept over the four fields
PostgreSQL can echo one into, all of which `server_error` drops) and `savepoints_nest` (§ 7's
nesting five deep, a name taken again once its level closed, and exactly one level of a run being a
real transaction under a mixture of commits and rollbacks).

**Five of the Stage 5 `[[check]]`'s seven names cannot live where its `args = ["test", "-p",
"nvs-db"]` looks** (`docs/agent/loop-goal.toml:2830`), so that check cannot go green from this
crate — the same wall Stage 4 hit. `a_transaction_is_a_closure_and_transaction_is_a_queryable`,
`roll_back_survives_an_intervening_catch_of_throwable` and `retries_recover_an_induced_deadlock` are
`nvs-stdlib`'s `Core\Db` surface and are the next group; `every_driver_normalises_its_codes_to_one_error_kind`
and `mariadb_uses_its_own_code_table_and_not_mysqls` wait on Stage 6's four drivers. **One widening
of `args` closes both Stage 4's and Stage 5's checks**, and that edit is the user's call, not a
session's.

**The driver's acceptance line still names `examples/queue.nvs`**, Stage 8's unlanded `Core\Queue`
(ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s module doc owns why.

## Next group

**Stage 5's three names that are `nvs-stdlib`'s, and the file set is `crates/nvs-stdlib/src/db.rs`
alone.** All three sit around the one helper, and the playbook's *a `-p nvs-stdlib` test can hand a
`Core` member a real `callable`* bullet is what makes them writable without a compiler in front.

- [ ] **`a_transaction_is_a_closure_and_transaction_is_a_queryable`** — § 7's closure form and the
      `Transaction implements Queryable by $connection` delegation, so the query surface is declared
      once. The row is `crates/nvs-stdlib/src/db.rs:488` and the helper
      `crates/nvs-stdlib/src/db.rs:3166`. ADR 0067 § 7.
- [ ] **`roll_back_survives_an_intervening_catch_of_throwable`** — § 7's rollback-only **flag**, which
      the owning frame acts on rather than on catching `Core\Db\RolledBack`, so an intervening
      `catch (Throwable)` cannot leave the transaction committed. The raise is
      `crates/nvs-stdlib/src/db.rs:3223` and the member `crates/nvs-stdlib/src/db.rs:3247`.
      ADR 0067 § 7.
- [ ] **`retries_recover_an_induced_deadlock`** — § 7's `{retries: n}`, on `Deadlock` and
      `SerializationFailure` only and outermost transactions only, defaulting to 0. **Check first
      whether the options bag reaches the helper at all**: the row at
      `crates/nvs-stdlib/src/db.rs:488` declares `{isolation?, readOnly?, retries?}` while
      `crates/nvs-stdlib/src/db.rs:3166` takes `args: [2]`, and the card at
      `crates/nvs-stdlib/src/db.rs:1500` already documents the default. ADR 0067 § 7.

## Backlog

- Stage 6's four drivers gate two Stage 5 names — `docs/agent/loop-goal.toml:2830`.
- Stage 4's four leftovers are the same `args` question — `docs/agent/loop-goal.toml:2806`.
- Stage 7's pool is `-p nvs-db` work with nothing on disk yet — `docs/agent/loop-goal.toml:2884`.
- `examples/queue.nvs` needs Stage 8's `Core\Queue` — ADR 0084.
- `open` waits on a shape-parameter type — `docs/implementation-plan.md`.
- The compose CA is not in git — `nvs_host::tls`'s module doc.
