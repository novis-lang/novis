# Handoff

## State

**Goal `m8-db-queue`, stage 5 is half closed.** Every driver keeps the version its handshake
delivered, and `Core\Db\Connection::serverVersion` answers it from memory with no round trip. The
field is on each of the four wire connections (`crates/nvs-db/src/conn.rs`); SQLite carries none by
design, because it has no handshake — `crates/nvs-db/src/sqlite.rs:675`'s `library_version()` is
what the enum's `server_version()` (`crates/nvs-db/src/conn.rs:1134`) answers for that arm. What
each driver answers is ADR 0187 § 2's table and nothing else decides it.

**One design call this session made, under ADR 0004's ordering.** A PostgreSQL startup that reports
no `server_version` is refused rather than kept as an empty string: the version is what
`rule:core-classes/schema-plan` grades against, so a blank one is a silent wrong answer where a
refusal is a loud one. Every PostgreSQL sends the parameter, and so does every pooler that forwards
startup parameters.

The member's three `.nvst` cases are on disk and the four in-crate driver tests are green. Nothing
is blocked; stage 1 is the carried floor and the driver's to run.

## Next group

**Stage 5: `streamAs`, and the roster hole that let `serverVersion` go unlisted** — one file set:
`crates/nvs-stdlib/tests/db_stream.rs`, `crates/nvs-stdlib/tests/spec_registry_coverage.rs` and
`crates/nvs-stdlib/src/db/stream.rs`.

- [ ] **`server_version_answers_what_the_connection_kept`**, the matrix half of the member: a leg
      answers a non-empty version, and it is the one the server reported. Write it beside
      `crates/nvs-stdlib/tests/db_stream.rs:387`, over the same `leg()` gate at
      `crates/nvs-stdlib/tests/db_stream.rs:75`, reading `nvs_db::Connection::server_version` at
      `crates/nvs-db/src/conn.rs:1134`. `rule:core-classes/db-one-api`, and ADR 0187 § 2's table for
      what each driver is entitled to say. Run it on all five legs with
      `python tools/db-matrix.py --all`.
- [ ] **`streamAs<T>` on every driver**, with `stream_as_answers_rows_at_the_class_it_was_written_with`
      in `crates/nvs-stdlib/tests/db_stream.rs` and the compile-time refusal case
      `tests/conformance/reject/db-stream-as-over-a-class-without-db-derive-is-refused-while-compiling.nvst`.
      It is `queryAs`'s hydration over the walk `crates/nvs-stdlib/src/db/stream.rs` already does —
      `rule:core-classes/db-streaming` for the member, `rule:core-classes/derive-attribute` for what
      makes a class eligible, and the goal's § *Standing decisions* for `E0806` being the code the
      last run-time refusal becomes (`crates/nvs-stdlib/src/db/row.rs:130-135` stays as the backstop).
- [ ] **`every_member_beyond_queryable_is_enumerated_from_the_spec`**, the gate that would have
      caught `serverVersion` missing: `crates/nvs-stdlib/src/db/registry.rs:448`'s
      `BEYOND_QUERYABLE` is hand-written, and nothing reads spec § 18's second table against it.
      Write it beside `crates/nvs-stdlib/tests/spec_registry_coverage.rs:1133`, over the parser at
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:1012` — `rule:core-api/tier-roster`.

## Backlog

- `crates/nvs-db/src/tds/token.rs:228` renders the version per call; a field on `TdsConn` holds it,
  so nothing caches — fine, but it is the one driver whose answer is built rather than kept.
- The `unowned` gaps in `crates/nvs-stdlib/src/db/mod.rs` and `crates/nvs-stdlib/src/queue.rs` are
  goal `unowned-closures`'s, per this goal's § *Standing decisions*.
