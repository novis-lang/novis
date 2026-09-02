# Handoff

## State

**Stage 2's § 2 check is closed, and the name now sits in the crate that can host it.**
`a_named_connection_is_memoized_for_the_request` is a `-p nvs-stdlib` unit test in
`crates/nvs-stdlib/src/db.rs`, and `docs/agent/loop-goal.toml` (mirrored into
`docs/agent/goals/5-database.toml`) reaches it through a stage 2 `nvs-stdlib (§ 2's memo …)` check of
its own rather than through the `-p nvs-db` block, per ADR 0132 § 1's crate edge.

**It asserts § 2 without a server, and the test's own doc comment owns how.** The context is set up as
the first `connect` leaves it — one connection filed under `main` — and then carries *no configuration
at all*, so a shared call answering a key is proof it returned at the memo, while `{shared: false}` on
the same context refuses for want of a `[db.main]` block. One connection is counted, not read off the
key: the next name filed lands at the very next slot.

**What is open is the rest of stage 2, and it is `crates/nvs-db/src/pg.rs`'s.** Both remaining names
are behaviours of the PostgreSQL driver itself, and neither has a test under its own name yet — the
charset one has a landed *assertion* inside another test, which is not what a `cargo-named` check
matches. `local_infile_is_refused_and_no_file_is_sent` still reports "did not run" permanently in this
goal-run: it is MySQL's, and stage 2 forbids a second driver until PostgreSQL is green end to end, so
closing the two below moves the ledger's report to that name rather than clearing it.

## Next group

**Stage 2's two remaining `-p nvs-db` names. Both are `crates/nvs-db/src/pg.rs` — the wire it writes
and the `mod tests` at the foot of the same file — so take them together.**

- [ ] **No driver path interpolates a value into SQL** — ADR 0067 § 1's "emulated prepares do not
      exist in any form", as `no_driver_path_interpolates_a_value_into_sql`. Every path that reaches
      the wire binds through the extended protocol: `crates/nvs-db/src/pg.rs:4139` is the landed
      single-statement assertion and `crates/nvs-db/src/pg.rs:4107` the `binds` helper that reads a
      flush back, `crates/nvs-db/src/pg.rs:2779` is `execute_many`'s wire half and
      `crates/nvs-db/src/pg.rs:681` its entry point. Assert it as a sweep over *every* way in —
      a value that would be catastrophic interpolated (a quote, a `--`, a `;`) reaches the server as a
      Bind parameter and never as statement text, so a path added later fails here.
- [ ] **The connection charset is forced to UTF-8** — ADR 0067 § 9's "connection charset forces
      UTF-8", as `the_connection_charset_is_forced_to_utf8`. The parameter is written at
      `crates/nvs-db/src/pg.rs:886`; `crates/nvs-db/src/pg.rs:3851` already asserts the startup bytes
      *inside* `scram_sha_256_authenticates_and_the_cancellation_key_survives_startup`, so the work is
      a test under the check's own name — do not rename that one, it pins the handshake. The second
      half is `crates/nvs-db/src/pg.rs:1821`, where a text field is validated as UTF-8 on the way in
      because, per `crates/nvs-db/src/pg.rs:104`, `client_encoding` is not what makes that safe.

## Backlog

- `local_infile_is_refused_and_no_file_is_sent` — MySQL's, and stage 2 forbids a second driver
  (`docs/agent/loop-goal.toml`, the stage 2 header).
- `open` waits on a shape *parameter* in the registry — `crates/nvs-stdlib/src/db.rs`'s module doc
  owns that gap.
- Stages 6-8's remaining names, `docs/agent/loop-goal.toml`.
