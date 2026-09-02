# Handoff

## State

**Stage 4's § 9 sweep is on disk in `crates/nvs-db/src/pg.rs`.** The type map is now a shared
`TYPE_MAP` const in that file's test module — one row per constant the `oid` module names, carrying
the OID, its modifier, a body, spec § 18's description and § 9's decoded value.
`every_row_of_the_type_map_round_trips` asserts the describe/decode round trip a row at a time plus
the two bounds the landed halves could not make: that both answers name the *same* row of § 9
(`row_of_the_type_map`, exhaustive on `ColumnType`), and that the sweep's length equals the number of
`pub(super) const` lines the `oid` module declares, counted out of this file's own source by
`oid_constants` — so a type the driver gains without a row here fails rather than passing quietly.
`no_driver_returns_a_number_as_a_string` sweeps that same const rather than a second list, and its
own rows are the values at which a string and a float stop being the same answer as a number.

**Three of Stage 4's six open names cannot be hosted where the check looks.**
`tinyint_one_reads_int_and_bool_and_throws_for_a_stored_seven` and
`bigint_unsigned_past_i64_max_reads_uint_and_throws_for_int` are MySQL/MariaDB rows of § 9 and wait
on Stage 6's drivers; `query_as_throws_naming_the_column_…` is `nvs-stdlib`'s `queryAs` while the
check's `args` is `-p nvs-db`. `changed` is the same shape and the group below works around it: the
member is `crates/nvs-stdlib/src/db.rs:4328`, and `crates/nvs-db/src/pg.rs:2469` holds the rule the
PostgreSQL half is.

**The driver's acceptance line still names `examples/queue.nvs`**, Stage 8's unlanded `Core\Queue`
(ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s module doc owns why.

## Next group

**The two Stage 4 names PostgreSQL can answer — the `[[check]]` block at
`docs/agent/loop-goal.toml:2806` — and the file set is `crates/nvs-db/src/pg.rs` alone.** Both go
beside the sweep that landed this session; `TYPE_MAP` is there to be reused rather than re-listed.

- [ ] **`affected_is_the_matched_count_and_changed_is_mysql_only`** — § 4's count on this protocol:
      the tag's number is the rows *matched*, and `changed` is that same number rather than a second
      one there is nothing to read. `crates/nvs-db/src/pg.rs:2475` is `affected`,
      `crates/nvs-db/src/pg.rs:2411` is `affected_rows`, and `crates/nvs-db/src/pg.rs:2469` is the
      doc stating the rule. The landed
      `the_affected_row_count_is_the_tags_last_field_and_a_ddl_tag_has_none`
      (`crates/nvs-db/src/pg.rs:5512`) already owns tag parsing, so this one asks the
      matched-versus-altered question over a scripted `UPDATE` and never the parse again. ADR 0067
      § 4.
- [ ] **`a_zoneless_column_reads_in_the_declared_zone_and_a_timestamptz_ignores_it`** — § 9's
      zone-less `DATETIME`/`TIMESTAMP` row against the row that carries its own offset.
      `crates/nvs-db/src/pg.rs:2117` is `timestamp(text, zoned)`,
      `crates/nvs-db/src/pg.rs:631` is the connection's `time_zone()`, and
      `crates/nvs-db/src/pg.rs:1780` is where the two OIDs split. ADR 0067 § 9.

## Backlog

- Stage 4's two MySQL rows wait on Stage 6's drivers — `docs/agent/loop-goal.toml:2811`.
- `query_as_throws_naming_the_column_…` is `nvs-stdlib`'s and the check is `-p nvs-db` —
  `docs/agent/loop-goal.toml:2822`.
- `a_decimal_into_a_float_field_throws` is ADR 0071's derive refusal — `crates/nvs-types/src/derive.rs`.
- The pool is Stages 5 to 7 — ADR 0067 § 13.
- The compose CA is not in git — `nvs_host::tls`'s module doc.
- `examples/queue.nvs` is Stage 8's `Core\Queue` — ADR 0084.
