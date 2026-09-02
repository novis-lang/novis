# Handoff

## State

**Stage 4 is complete as far as `-p nvs-db` can take it**, all of it in `crates/nvs-db/src/pg.rs`:
§ 9's swept type map, § 4's affected count (`affected_is_the_matched_count_and_changed_is_mysql_only`
— the tag's number is the rows matched, and the driver carries no second count for `changed` to
read, asserted by scanning this file's own source) and § 9's two timestamp rows
(`a_zoneless_column_reads_in_the_declared_zone_and_a_timestamptz_ignores_it` — the decode does not
move with the connection's declared zone, over a sweep of the zones a block can write).

**The four names left in the Stage 4 `[[check]]` (`docs/agent/loop-goal.toml:2806`) cannot be
hosted where its `args = ["test", "-p", "nvs-db"]` looks**, so that check cannot go green from this
crate: `tinyint_one_reads_int_and_bool_and_throws_for_a_stored_seven` and
`bigint_unsigned_past_i64_max_reads_uint_and_throws_for_int` are MySQL/MariaDB rows waiting on Stage
6's drivers; `query_as_throws_naming_the_column_…` is `nvs-stdlib`'s `queryAs`; and
`a_decimal_into_a_float_field_throws` is the *compile-time* `E0756` raised at
`crates/nvs-types/src/derive.rs:548` and already asserted at `crates/nvs-types/tests/derive.rs:45`,
whose runtime twin is `nvs-stdlib`'s typed `Row` readers. Either the check's `args` widen or Stage 6
lands; nothing in `nvs-db` closes it.

**The driver's acceptance line still names `examples/queue.nvs`**, Stage 8's unlanded `Core\Queue`
(ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s module doc owns why.

## Next group

**Stage 5's two names `-p nvs-db` can answer — the `[[check]]` block below Stage 4's in
`docs/agent/loop-goal.toml` — and the file set is `crates/nvs-db/src/pg.rs` alone.** Both sit beside
cases already in that module, so nothing new has to be scripted from scratch.

- [ ] **`a_db_error_message_contains_no_bound_value`** — § 8's rule that a refusal quotes the
      server's sentence and never a parameter, so a message can be logged. `server_error` is
      `crates/nvs-db/src/pg.rs:1007`, the scripted refusals are `crates/nvs-db/src/pg.rs:3949`
      (`error_response`) and `crates/nvs-db/src/pg.rs:3931` (`constrained_error_response`), and the
      landed `a_refused_statement_carries_section_8s_kind_beside_the_sentence`
      (`crates/nvs-db/src/pg.rs:5229`) already owns the kind, so this one binds a value the server
      then echoes and asserts the value is nowhere in what the driver raised. ADR 0067 § 8.
- [ ] **`savepoints_nest`** — § 7's nesting past depth two, which the two landed cases stop short
      of: `crates/nvs-db/src/pg.rs:5095` names one savepoint by depth and
      `crates/nvs-db/src/pg.rs:5126` releases it, so this one drives three deep and asserts the
      names and the release order over one connection. `savepoint_name` is
      `crates/nvs-db/src/pg.rs:3127`. ADR 0067 § 7.

## Backlog

- `every_driver_normalises_its_codes_to_one_error_kind` and `mariadb_uses_its_own_code_table_and_not_mysqls` — Stage 6's drivers; `docs/agent/loop-goal.toml`.
- `a_transaction_is_a_closure_and_transaction_is_a_queryable` and `roll_back_survives_an_intervening_catch_of_throwable` — the language surface, not `-p nvs-db`.
- `retries_recover_an_induced_deadlock` — decide where § 8's retry lives before writing it; ADR 0067 § 8.
- Stage 4's four un-hostable names — widen the check's `args` or wait on Stage 6; `docs/agent/loop-goal.toml`.
- `open` waits on a shape-parameter type — `docs/plan/m8.md`.
- The compose CA is not in git — `crates/nvs-host/src/tls.rs`'s module doc.
