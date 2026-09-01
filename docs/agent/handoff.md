# Handoff

## State

**Goal 4, M8.** The acceptance failure this session opened on — `valgrind examples/limits.nvs: exit 1`
with the fixture's own `FATAL` on stderr — was a **tooling bug, not a regression**: `[valgrind] skip`
lost `examples/limits.nvs` at the goal-3→goal-4 switch, because `tools/goal-switch.py`'s `union_list`
matched only a multi-line list and `skip` is written on one line. The fixture is back in the skip list
with the comment that says why the list exists, and the union now matches both list shapes and refuses
rather than silently carrying nothing (playbook, *Tooling*).

**`mysqli` is fully classified.** `docs/spec/02-php-migration.md:1375` is a new
`## Databases: the `mysqli` extension` — a lead naming the three rules that empty it
([ADR 0067](../adr/0067-core-db.md) §§ 1, 2, 7 plus [ADR 0063](../adr/0063-core-api-conventions.md) R17)
and four `###` tables covering all 106 names. `python tools/check-migration.py --min 74` is at **90%**,
1,032 of 1,152 classified, 120 open.

**Those 120 open names are one family: `pg_*`, and nothing else.** `sqlite3` contributes no function to
the inventory at all — its whole surface is methods on `SQLite3`/`SQLite3Stmt`/`SQLite3Result`, which are
types — so `pgsql` is the last of the migration table. Nothing about § 15's stdlib half moved: ADR 0112's
compile-time half is still absent (`crates/nvs-stdlib/src/cap.rs`'s module doc owns what that costs) and
`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is still 41 keys.

## Next group

**All three slices are `docs/spec/02-php-migration.md` alone**, appending `###` tables to a new
`## Databases: the `pgsql` extension` placed immediately before `## Not yet classified`.
[01 § 18](../spec/01-core-library.md) is the signature roster the cells name, and `check-migration.py`
validates every `Core\...::member` spelling in a cell against it — so run it after each slice, not at the
end. The `mysqli` section directly above is the shape to copy: a lead that names the ADR rules doing the
work, then one table per family.

- [ ] **Migration rows: `pg_*`, connections and the calls that run a statement** (~60).
      `pg_connect`/`_pconnect`/`_close`/`_connection_status`/`_connection_busy`/`_ping`/`_host`/`_dbname`
      against ADR 0067 § 2's naming and memoization and § 13's per-core pool and its reset;
      `pg_prepare`/`_execute`/`_query`/`_query_params`/`_send_*`/`_get_result` against § 1's "there is no
      `prepare`" and § 4's five members; `pg_escape_string`/`_literal`/`_identifier`/`_bytea` against
      § 12's permanent refusal, with `Core\Db::quoteIdentifier` the one survivor.
      `docs/spec/02-php-migration.md:1528`, `docs/spec/01-core-library.md:1140`.
- [ ] **Migration rows: `pg_*`, results, transactions and the deferred subsystems** (~60).
      `pg_fetch_*`/`_num_rows`/`_num_fields`/`_field_*`/`_result_*` against § 6 and 01 § 18's *Results*
      table; `pg_lo_*` (~15) against § 12's deferred LOB streaming; `pg_copy_to`/`_from` against its
      deferred `COPY`; `pg_get_notify`/`_get_pid` against deferred `LISTEN`/`NOTIFY`;
      `pg_meta_data`/`_convert`/`_insert`/`_update`/`_delete`/`_select` against ADR 0051 test 6, which
      puts a query builder outside `Core` entirely.
      `docs/spec/02-php-migration.md:1528`, `docs/spec/01-core-library.md:1186`.
- [ ] **Close the table.** With `pg_*` landed the inventory is at 100% and
      `## Not yet classified`'s closing paragraph is wrong — it now names `pgsql` as the last extension.
      Rewrite it to say what the section still means (the unaudited-extension hole and
      `AHEAD_OF_THE_BUILD` are not the same thing as an unclassified name), and raise
      `check-migration`'s `--min` gate in `docs/agent/loop-goal.toml` from 74 to match.
      `docs/spec/02-php-migration.md:1528`, `docs/agent/loop-goal.toml`.

## Backlog

- ADR 0112's compile-time half is absent — `crates/nvs-stdlib/src/cap.rs`'s module doc.
- `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is 41 keys — spec Part II.
- ADR 0067's five-driver CI-container matrix needs a reachable Docker daemon — plan, *Blocking*.
- `Core\Db` has no implementation at all yet; the spec and the migration table are ahead of it — M8.
