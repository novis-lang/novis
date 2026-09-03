# Handoff

## State

**M8 goal 5, stage 7. SQL Server's values now cross the wire in both
directions, and nothing above `nvs-db` can see them yet.**
`crates/nvs-db/src/tds.rs:4616`'s `encode` renders a bound parameter as T-SQL reads it — MySQL's
`1`/`0` for a `bit`, MySQL's refusal for a non-finite `float`, and a **refusal for a `bytes`**,
which is § 9's one open row on this driver: every parameter goes out as one `nvarchar` and
`varbinary` has no text form a cast recovers, so the gap closes with a marker of its own or not at
all. `crates/nvs-db/src/tds.rs:3692`'s `scalar` is the other direction — `TdsScalar` and its
`TdsDate`/`TdsTime`, `crate::PgScalar`'s shape with no `UInt` and no `Array` row — and
`decode_column` beside it is `crate::mysql::decode`'s twin under a longer name, because this module
writes its own framing and `decode` is already the packet reader.
`crates/nvs-db/src/tds.rs:5014`'s `TdsConn::query` is the two-line delegation; `execute` is that
same method, since `sp_prepexec` carries both and `TdsRows::affected` is what parts them.

**Three of § 9's rows are computed and not copied**, and each is silent when got wrong: `money`
puts its high four bytes first, `uniqueidentifier`'s first three groups are little-endian on the
wire and big-endian in every text form, and `datetimeoffset` stores UTC with the offset beside it
where `timestamptz` arrives already shifted. `money_a_guid_and_an_offset_are_not_read_the_way_their_bytes_are_laid_out`
asserts each against the reading a straight copy would have given.

**A non-UTF-8 `varchar` is refused rather than transcoded.** SQL Server has no session charset to
force — ADR 0067 § 3's guarantee is `utf8mb4` on one protocol and nothing on this one — so a
collation that is not UTF-8 has no `string`, and `scalar`'s doc owns why that beats a character
table. `sql_variant` and a CLR type are refused for the same reason.

**Nothing above the driver is wired.** `crates/nvs-stdlib/src/db.rs:4303`'s `rendering_for` still
answers `None` for `Driver::SqlServer`, so `queried_rows` still falls to `driverless` — that is the
next group, and it is the whole of what stands between this and § 4 running.

**The driver's standing acceptance failure is stage 9's, and it is not this goal's to close yet.**
`an_open_host_matching_no_grant_is_a_diagnostic` is `nvs_types::intrinsics`' known gap 6: checking
has no capability configuration in front of it at all — `Env` carries no capability set and no
capability is checked at check time, `net.connect` included. The list's other missing name,
`a_tainted_settings_host_is_a_diagnostic_naming_assert_trusted`, is gated on the *first two* of
that gap's three and one of those has since closed (see Backlog).

## Next group

**One file set: `crates/nvs-stdlib/src/db.rs`, with `crates/nvs-db/src/tds.rs` for what it calls.**
`nvs_db::Connection::MySql`'s arms are the shapes to copy throughout, and the whole group is one
question — what a `Driver::SqlServer` connection does when § 4 asks it for rows.

- [ ] **`rendering_for` gains `Driver::SqlServer`, and `queried_rows` gains its arm**
      (0067 §§ 4 and 5). `crates/nvs-stdlib/src/db.rs:4303`, `crates/nvs-stdlib/src/db.rs:4564`,
      `crates/nvs-stdlib/src/db.rs:4598`, `crates/nvs-db/src/tds.rs:5014`. The encoder is
      `nvs_db::tds::encode` and the dialect is `Dialect::SqlServer`; `driverless`'s message names
      the shrinking list and has to shrink with it.
- [ ] **A `tds_column_value` turns § 9's five structured rows into their `Core` instances**
      (0067 § 9). `crates/nvs-stdlib/src/db.rs:5267`, `crates/nvs-db/src/tds.rs:3642`,
      `crates/nvs-db/src/tds.rs:3692`. `column_value`'s `PgScalar` match is the shape;
      `TdsScalar`'s date and time carriers hold the same fields `crate::time::date_at` takes.
- [ ] **The pool's reset and destroy arms reach `TdsConn::reset`** (0067 § 13).
      `crates/nvs-stdlib/src/db.rs:4174`, `crates/nvs-db/src/tds.rs:5014`. `MySqlConn::reset`'s arm
      one line above is the shape, and `TdsConn::reset` already consumes itself the same way.

## Backlog

- `nvs_types::intrinsics`' known gap 6 is stale in its first clause: `Core\Db::open` **does** have
  a registry row now (`crates/nvs-stdlib/src/db.rs:619`), so only the last two of its three are
  still missing. One edit to that module doc, at `crates/nvs-types/src/intrinsics.rs:70`.
- § 9's `bytes` on SQL Server: a `varbinary` parameter marker beside `text_param`, or the refusal
  pinned as the answer in a `.nvst` case — `crates/nvs-db/src/tds.rs:4616` owns the argument.
- `executeMany` and § 7's `transaction` have no SQL Server arm; `crates/nvs-stdlib/src/db.rs:5587`
  and `crates/nvs-stdlib/src/db.rs:4472` are where they would go.
- Stage 9's `nvs-types` check is a seven-name conjunction with two names unwritable for different
  reasons; splitting it names the work honestly — `docs/agent/loop-goal.toml:3111`.
- `docs/agent/loop-goal.toml`'s `[context]` printed everything this session needed.
