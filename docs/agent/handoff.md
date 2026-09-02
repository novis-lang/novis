# Handoff

## State

**Stage 2's two PostgreSQL names are closed, both in `crates/nvs-db/src/pg.rs`'s `mod tests`.**
`no_driver_path_interpolates_a_value_into_sql` sweeps every way a value reaches the wire — `query`
cached and uncached, `executeMany` cached and uncached — and reads the flushes back for the *tag of
every message carrying the value*, asserting each is a `Bind` and counting one per execution. A path
added later that renders a value into statement text fails it without anyone remembering to look.

`the_connection_charset_is_forced_to_utf8` asserts both halves of ADR 0067 § 9's charset row: the
startup message's parameter list is parsed into pairs and `client_encoding` is `UTF8` exactly once,
and every OID that reads back as text refuses a Latin-1 body while `BYTEA` accepts the same bytes.
The `client_encoding` assertion that used to sit inside
`scram_sha_256_authenticates_and_the_cancellation_key_survives_startup` moved here; that test keeps
§ 9's `DateStyle` and `TimeZone`, which are the handshake's.

**Stage 2's check still fails, and permanently.** Its remaining name,
`local_infile_is_refused_and_no_file_is_sent`, is MySQL's, and the stage forbids a second driver
until PostgreSQL is green end to end — stage 5 still has three names open. The ledger's report is
that name from now on; it is not a regression and no session should try to close it.

## Next group

**Stage 7's reset half — § 13's reset is the security boundary, and all three are unit tests over
landed code in `crates/nvs-db/src/pg.rs`: `reset_session` and the four reset cases already in that
file's `mod tests`.**

- [ ] **PostgreSQL resets without losing its statement cache** — ADR 0067 § 13's "deliberately not
      `DISCARD ALL`, which also deallocates prepared statements", as
      `postgres_resets_without_losing_its_statement_cache`. `crates/nvs-db/src/pg.rs:2958` is
      `reset_session` and `crates/nvs-db/src/pg.rs:5187` the landed assertion that its six commands
      are one flush. It takes no cache, so the claim is asserted around it: `start_statement` over a
      `StatementCache` twice with a reset in between, the second still a cache hit (`tags` shows
      `BDES`, no `P`), and no `DEALLOCATE`/`DISCARD` in the reset's own flush.
- [ ] **A failed reset destroys the connection** — § 13, as
      `a_failed_reset_destroys_the_connection_rather_than_returning_it`.
      `crates/nvs-db/src/pg.rs:5223` and `crates/nvs-db/src/pg.rs:5250` already pin that a refused
      reset reports and that a wire failure poisons; what has no name is that the connection is
      *dropped* rather than handed back. If the drop is the pool's, the pool is
      `crates/nvs-stdlib/src/db.rs` and this is the § 2 memo's crate edge again — decide it, move
      the name to a `-p nvs-stdlib` check of its own as ADR 0132 § 1 did, and say so.
- [ ] **No session state survives a return to the pool** — § 13's property list (no transaction, no
      temp table, no session variable, no `SET ROLE`, no advisory lock, no listener, no open
      cursor), as `no_session_state_survives_a_return_to_the_pool`. Assert it as a sweep over that
      list against the commands `crates/nvs-db/src/pg.rs:2958` writes, counted, so a property with
      no command fails — `crates/nvs-db/src/pg.rs:5187` reads the flush back already.

## Backlog

- `local_infile_is_refused_and_no_file_is_sent` — MySQL's, permanent stage 2 report above.
- `the_pool_is_per_core_and_keyed_as_connect_and_open_key` — stage 7's fourth name, and its `args`
  is `-p nvs-db` while the pool is `crates/nvs-stdlib/src/db.rs`'s.
- `retries_recover_an_induced_deadlock` and `every_driver_normalises_its_codes_to_one_error_kind` —
  stage 5's open names; the retry ladder is § 7's closure in `nvs-stdlib`, `kind_of` is
  `crates/nvs-db/src/pg.rs:1073`.
- Stage 9's seven `check and trace` names — all diagnostics, none in the tree yet.
- `open` waits on a shape *parameter* in the registry — `crates/nvs-stdlib/src/db.rs`'s module doc
  owns that gap.
