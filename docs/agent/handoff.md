# Handoff

## State

**Stage 5's `stream` item is landed; the three that remain are unlanded *features*, not tests over
landed work.** `crates/nvs-stdlib/src/db/stream.rs` now carries both of the pair's named tests, and
`park_row` — `stream_step`'s tail, split out — is where ADR 0067 § 4's "one row at a time whatever
the result set's size" is kept and counted. The count is over a thousand rows and it is a real
gate: retaining the displaced row makes it fail with every index instead of the last.

The other half of that item is deliberately in `nvs-db`: nothing in `nvs-stdlib` can build a
connection, so `stream_next_row` is pinned there and this crate pins the two things it owns — the
parking, and the mapping of the driver's busy `InvalidInput` onto § 4's `LogicError` at every member
that can be the second statement.

Stage 5's remaining three checks each need a feature first, and the triage for two of them is below
rather than in the item, because it was a grep each and the next session should not pay for it
again. Nothing is blocked on a decision: both open design calls are pre-authorized by the goal's
§ *Standing decisions* items 10 and 12.

## Next group

**Stage 5 — the three `Core\Db` checks that need a feature before they need a test.** One file set:
`crates/nvs-stdlib/src/db/open.rs`, `crates/nvs-stdlib/src/db/pool.rs`,
`crates/nvs-config/src/db.rs`, `crates/nvs-config/src/tree.rs`, `docs/adr/0067-core-db.md`.

- [ ] **Item 10's pool answer lands**, per the goal's § *Standing decisions*. The config half already
      exists — `nvs_config::db::pool_for` resolves a block's `[db.<name>.pool]` — and the two sites
      that throw it away are `crates/nvs-stdlib/src/db/open.rs:678` and
      `crates/nvs-stdlib/src/db/open.rs:884`, both passing `PoolBounds::DEFAULT` to
      `Ticket::for_settings`; `connect`'s own site at `crates/nvs-stdlib/src/db/open.rs:272` is
      already right and is the shape to copy. The block lookup is by `settings_key`, built at
      `crates/nvs-stdlib/src/db/open.rs:661`, so matching a block means building that key from the
      block's own resolved fields. **The unscoped `pool = false` is the harder half and has a
      spelling to decide**: `crates/nvs-config/src/tree.rs:98` is `pub db: BTreeMap<String,
      Database>`, so there is no home in that map for a key that is not a block name — either a
      reserved name in the map or `db` becomes a struct with a `pool` field and a flattened map, and
      the second is the one that cannot collide with a block an operator named.
      `an_open_reads_its_pool_bounds_from_the_blocks_pool_table` and
      `an_unscoped_pool_false_reaches_a_program_opened_connection`, at
      `crates/nvs-stdlib/src/db/pool.rs:289`.
- [ ] **ADR 0067 § 13's amendment is folded into the ADR body** once the slice above lands —
      `python tools/adr.py --fold` writes both halves and the body edit is the half no tool does —
      `docs/adr/0067-core-db.md:1`, `crates/nvs-stdlib/src/db/pool.rs:289`.
- [ ] **A statement timeout reaches the socket**, and the first question is which timeout it is.
      `timeout` appears in ADR 0067 exactly once, at `docs/adr/0067-core-db.md:81`, as `connect`'s
      and `open`'s *call-level* option — there is no statement timeout decided anywhere, and the word
      appears in neither `db/execute.rs` nor `db/bind.rs`. What already holds is the reaching: all
      four wire drivers set the socket's deadline at connect (`crates/nvs-db/src/pg.rs:478`,
      `crates/nvs-db/src/mysql.rs:1421`, `crates/nvs-db/src/maria.rs:364`,
      `crates/nvs-db/src/tds/mod.rs:392`) and **no `set_deadline(None)` clears it afterwards**, which
      is worth checking before writing anything: a pooled connection whose connect deadline has
      passed would refuse its next statement on the clock rather than on the server.
      `a_statement_timeout_reaches_the_socket_and_throws_on_expiry` —
      `crates/nvs-stdlib/src/db/execute.rs:1287`, `crates/nvs-config/src/db.rs:180`.

## Backlog

- Stage 5's other three checks are green or already landed; `close_releases_the_connection_and_a_later_member_refuses` is in `crates/nvs-stdlib/src/db/open.rs`.
- `nvs_stdlib::db`'s known gap 5 — only PostgreSQL parks a cursor, so `stream` throws on the other four (`crates/nvs-stdlib/src/db/stream.rs`'s `unstreamed`).
- The `[context]` manifest wanted nothing this session that it did not print.
