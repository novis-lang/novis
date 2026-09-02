# Handoff

## State

**ADR 0067 § 4's streaming half is asserted**, which closes the two `nvs-db` names Stage 3's
`cargo-named` check lists for it. `a_large_result_streams_at_constant_memory` reads the bound off
`nvs_runtime::budget` while a hundred thousand rows go past — a row count is what a *buffering* driver
passes too, so the heap while the rows move is the whole claim.
`a_second_statement_on_a_busy_connection_is_a_logic_error` pins the classification rather than the
refusal the case beside it already pins: one predicate decides it, and the connection is still usable
once the caller reads its rows.

**A stream needs a server that does not buffer either.** `Peer`, this module's scripted server, keeps
every byte it ever answered in `inbound`, so a large result read over it would measure the harness.
`Firehose` generates one message at a time into the buffer the last one used, and its doc comment owns
why the bound has to hold on both sides of the stream — server and driver are one thread and one heap.

**Stage 3's four remaining names are § 5's rewriter and none of them exists yet**; one has a landed
near-twin under another name, which the next group's last item spells out.

**The driver's acceptance line still names `examples/queue.nvs`**, which is Stage 8's unlanded
`Core\Queue` (ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s module
doc owns why.

## Next group

**Stage 3's four remaining `nvs-db` names — ADR 0067 § 5's placeholder rewriter — and the file set is
`crates/nvs-db/src/sql.rs` alone.** Nothing in `nvs-stdlib` moves. **Read the `[[check]]` block at
`docs/agent/loop-goal.toml:2789` before writing anything**: its names are the specification, and the
playbook's bullet on stage 2's comment header forbids renaming a landed test into one of them.

- [ ] **`the_rewriter_skips_string_literals_and_comments`** — § 5's naive-scanner case: a `:name` or a
      `?` inside a string literal or a `--`/`/* */` comment is text, not a placeholder.
      `crates/nvs-db/src/sql.rs:423` is `rewrite` and `crates/nvs-db/src/sql.rs:785` opens the test
      module. ADR 0067 § 5.
- [ ] **`a_postgres_cast_and_a_jsonb_question_mark_survive_the_rewrite`** — the same scanner's two
      PostgreSQL spellings that look like the driver's own syntax, `::text` and `?`/`?|`/`?&`, in one
      query. `crates/nvs-db/src/sql.rs:423`. ADR 0067 § 5.
- [ ] **`a_named_parameter_used_twice_binds_one_value_once`** — the arity § 1's statement cache keys on
      is the *bind* count, so a name repeated in the SQL is one value and one slot.
      `crates/nvs-db/src/sql.rs:423`. ADR 0067 § 5.
- [ ] **`in_list_expands_and_an_empty_list_throws`** — `crates/nvs-db/src/sql.rs:882` already holds
      `in_list_expands_to_a_parenthesised_run_and_moves_the_arity`, which pins the expansion half only;
      the empty list's throw is what this adds, so it is a new case beside that one and never a rename.
      ADR 0067 § 5.

## Backlog

- `stream`'s registry rows in `nvs-stdlib` — a cursor that holds the connection past the call, ADR 0067
  § 18.
- Stage 4's § 9 type-map names under `-p nvs-db` — `docs/agent/loop-goal.toml`, stage `4 type map`.
- `Core\Db::open` waits on a shape-parameter type — `docs/implementation-plan.md`, *Open now*.
- `Core\Queue` is Stage 8 (ADR 0084) and is what the acceptance line names every iteration.
- The compose CA is still not in git — `crates/nvs-host/src/tls.rs`'s module doc owns why.
