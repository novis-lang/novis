# Handoff

## State

**Stage 7 is closed as far as it can be.** Every name in its `-p nvs-db` check now exists: the four
integration names were already in `crates/nvs-db/tests/pool_reuse.rs`, and the two this session added
are `a_failed_reset_destroys_the_connection_rather_than_returning_it`
(`crates/nvs-db/src/pg.rs:5351`) and `the_pool_is_per_core_and_keyed_as_connect_and_open_key`
(`crates/nvs-db/src/conn.rs:658`).

The reset case asserts § 13's "a connection that cannot be proven clean is closed" in two halves,
because the enforcement is a type and the destruction is a write: `PgConn::reset` coerced to
`fn(PgConn) -> io::Result<PgConn>` is the by-value signature, and `say_goodbye` — `Drop`'s body,
extracted as a free function generic in the stream for the reason the playbook gives about `PgConn`'s
`Wire` — puts the five bytes of `Terminate` on a scripted wire and nothing after them. The pool case
asserts § 2's key over the type actually filed, including the `into_any` downcast.

**Stage 7's check still fails and always will**, on
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs`: a second and third driver, the same
permanent shape as stage 2's `local_infile_is_refused_and_no_file_is_sent`. Both are backlog, not
work. **Stage 8's queue names all exist**; the first stage with open work is stage 9.

## Next group

**Stage 9's literal-query diagnostics — the file set is `crates/nvs-types/src/intrinsics.rs` and
`crates/nvs-types/tests/intrinsics.rs`. Read that module's own doc first: § 1's `INTRINSICS` table is
the whole dispatch, and `report_malformed` is the shared reporter — so none of these needs a new
diagnostic code, which matters because the `E04xx` and `E07xx` bands are both FULL.** The pack did not
print ADR 0067 § 2 or § 10 and prints no `nvs-types/src/intrinsics.rs` map line; add `0067` §§ 2 and
10 to `[context] adrs` and that module to `[context] modules`.

- [ ] **A `Core\Db` row and its SQL grammar** — ADR 0067 § 5's one placeholder spelling in, as
      `a_placeholder_count_mismatch_on_a_literal_is_a_diagnostic` and
      `mixed_placeholder_styles_on_a_literal_are_a_diagnostic`.
      `crates/nvs-types/src/intrinsics.rs:107` is the table to add the row to,
      `crates/nvs-types/src/intrinsics.rs:190` the `match row.grammar` needing the arm, and
      `crates/nvs-types/src/intrinsics.rs:237` is `check_template`, the shape a `check_sql` copies.
      The cases go beside `crates/nvs-types/tests/intrinsics.rs:28`'s `check_call(body)` helper.
- [ ] **Two statements in one literal query** — § 1's "every statement is prepared" has no spelling
      for two, as `a_two_statement_literal_query_is_a_diagnostic`. Same row and same arm:
      `crates/nvs-types/src/intrinsics.rs:190`, `crates/nvs-types/tests/intrinsics.rs:28`.
- [ ] **The taint pair, checked together** — ADR 0024 § 4's rule that the query-text parameter refuses
      `tainted` while a bound parameter accepts it freely, as
      `a_tainted_value_at_a_query_text_parameter_is_a_diagnostic` and
      `the_same_tainted_value_at_a_bound_parameter_compiles`. Check first whether the registry row's
      `Qual` already refuses it and only the assertion is missing; `crates/nvs-types/src/intrinsics.rs:164`
      is `check_call`, which already holds the argument types, and the cases go at
      `crates/nvs-types/tests/intrinsics.rs:28`.

## Backlog

- `mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` — waits on the MySQL and SQL Server
  drivers (`docs/agent/loop-goal.toml:2919`).
- `local_infile_is_refused_and_no_file_is_sent` — stage 2's, MySQL's, same wait.
- `an_open_host_matching_no_grant_is_a_diagnostic` and
  `a_tainted_settings_host_is_a_diagnostic_naming_assert_trusted` — both are about `Core\Db::open`,
  which waits on a shape-parameter type (the plan's *Open now*).
- Stage 9's `a_query_span_contains_no_parameter_value_anywhere` already exists in `-p nvs-db`.
