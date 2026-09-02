# Handoff

## State

**Stage 3's one `cargo-named` check is green**: all six names it lists for `nvs-db` now exist and
pass. The four § 5 rewriter names landed this session in `crates/nvs-db/src/sql.rs`, each asking a
question its landed near-twin does not — `the_rewriter_skips_string_literals_and_comments` sweeps
every hiding construct and asserts the *count* of binds rather than one line per construct;
`a_postgres_cast_and_a_jsonb_question_mark_survive_the_rewrite` puts `::`, `?|` and `??` in one
statement because they are one scan; `a_named_parameter_used_twice_binds_one_value_once` asserts the
reuse as an agreement across all four dialects, tying the arity to what § 1's cache keys on; and
`in_list_expands_and_an_empty_list_throws` names the length bound on both sides and pins the
refusal's `InvalidInput` classification. None was a rename of a green test — the playbook's bullet
on stage 2's comment header forbids that, and all six landed names are still there.

**Stage 4's eight names are next and none of them exists**, but two landed near-twins split the
sweep in half: `every_scalar_row_of_the_type_map_decodes_to_its_novis_type`
(`crates/nvs-db/src/pg.rs:5453`) and `every_structured_row_of_the_type_map_decodes_to_its_components`
(`:5541`). The new names have to ask across both halves, not re-ask either.

**The driver's acceptance line still names `examples/queue.nvs`**, which is Stage 8's unlanded
`Core\Queue` (ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s
module doc owns why.

## Next group

**Stage 4's § 9 type-map names — the `[[check]]` block at `docs/agent/loop-goal.toml:2806` — and the
file set is `crates/nvs-db/src/pg.rs` alone.** Nothing in `sql.rs` moves. Read that block first: its
names are the specification, and a landed near-twin may not be renamed into one of them.

- [ ] **`every_row_of_the_type_map_round_trips`** — § 9's whole table in one sweep, asserted by
      count across both halves the two landed tests split: every OID this driver names describes,
      decodes, and comes back as the Novis type the table gives it.
      `crates/nvs-db/src/pg.rs:1667` is `describe`, `crates/nvs-db/src/pg.rs:1720` is `decode`, and
      `crates/nvs-db/src/pg.rs:5331` is the `column()` helper the tests build one with. ADR 0067 § 9.
- [ ] **`no_driver_returns_a_number_as_a_string`** — § *Context*'s own defect: no numeric OID
      (`INT2`/`INT4`/`INT8`/`OID`/`FLOAT4`/`FLOAT8`/`NUMERIC`/`MONEY`) decodes to a text value, and
      `decimal` is a `decimal` rather than PHP's string. `crates/nvs-db/src/pg.rs:1720` and the
      `rendered()` helper at `crates/nvs-db/src/pg.rs:5406`. ADR 0067 § 9.
- [ ] **`affected_is_the_matched_count_and_changed_is_mysql_only`** — § 4's count: PostgreSQL's tag
      reports matched rows and there is no second `changed` number to read.
      `crates/nvs-db/src/pg.rs:2411` is `affected_rows`, `crates/nvs-db/src/pg.rs:2475` is
      `affected`, and the landed twin is `crates/nvs-db/src/pg.rs:5512`. ADR 0067 § 4.

## Backlog

- Stage 4's five other names — the `TINYINT(1)`, `BIGINT UNSIGNED`, `decimal`-into-`float`, declared-zone
  and `queryAs` ones; `docs/agent/loop-goal.toml:2806`.
- Stage 5 to 7: transactions, the pool, the four remaining drivers; `docs/agent/loop-goal.md`.
- `Core\Queue` (ADR 0084) is Stage 8 and owns the standing acceptance failure.
- The compose CA is generated, not committed; `crates/nvs-host/src/tls.rs`'s module doc.
