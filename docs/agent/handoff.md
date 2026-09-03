# Handoff

## State

**M8 goal 5, stage 8. SQLite is the fifth driver and Novis reaches it** — `crates/nvs-db/src/sqlite.rs`
holds § 4's statements, § 7's nesting, § 8's kinds, § 9's column map and § 13's reset, and
`crates/nvs-stdlib/src/db.rs` now opens it from both entry points and runs § 4's three members over
it. `tests/conformance/core/db-a-sqlite-block-opens-a-file-and-runs-statements-over-it.nvst` is the
only case in that tree that reaches a *running* database — `:memory:` needs no server — so the whole
chain is pinned end to end rather than at the type level.

**A parameter is a storage class on this driver and octets on the other four**, which is the one
shape difference that reached `nvs-stdlib`. `Encoder` is an enum over the two encoder signatures and
`Binds` is the same split on the result (`crates/nvs-stdlib/src/db.rs:4472`); `rendering_for` is
total now, so known gap 2's *binding* half is closed and what is left of that gap is § 7 alone.
`nvs_db::sqlite::encode` argues its four decisions — `bool` as `1`/`0`, a `uint` past `i64::MAX`
refused, `NaN` refused where the infinities are not, and `decimal` as `TEXT` with the column's
affinity free to round it.

**§ 13 pools SQLite now**: `warm_connection` resets it like the rest, `SqliteConn::reset` asking the
engine rather than the depth count.

**§ 2's `open` asks three capabilities of the path** — `db.open`, then `fs.read` and `fs.write` —
which is § 3's "additionally needs" read literally, with `db.open` scoped to the path because that
arm has no host for a host-shaped grant to be about. That is a decision this session made and
`sqlite_settings`' own doc is where it is argued.

**What a SQLite `query` does not do is § 9's declared-type map.** A cell arrives as the class it was
stored in and the column's `COLUMN` carries the declared answer beside it; turning the `TEXT` in a
`date` column into a `Core\Time\Date` is `Core\Db\Row`'s typed reader under § 6, and that is the next
item.

**A `[db.<name>] path` is not resolved against the config file's directory.** `nvs_config::db`
resolves `tls_ca_file` and nothing else, so a relative `path` is read against the process working
directory — while `SqliteTarget::path`'s own doc claims ADR 0103 § 5's resolution "has already
happened". One of the two is wrong and it is the tree; the backlog names it.

The standing acceptance failure is still stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`,
`nvs_types::intrinsics`' known gap 6, and is not this goal's to close.

## Next group

**One file set: `crates/nvs-stdlib/src/db.rs`**, with `crates/nvs-db/src/sqlite.rs` read-only.

- [ ] **§ 9's typed readers over a SQLite cell**, so a column declared `date`, `datetime`, `uuid`
      or `decimal` answers its `Core\Time`/`Core\Uuid`/`decimal` value and a cell that does not parse
      throws (0067 §§ 6, 9). `crates/nvs-stdlib/src/db.rs:7250` (`converted`, where § 6's requested
      type drives the conversion), `crates/nvs-stdlib/src/db.rs:5569` (`sqlite_column_value`, which
      is deliberately the storage class and says so), `crates/nvs-db/src/sqlite.rs:315`
      (`SqliteColumn::column_type`, the declared-type map the reader has to agree with).
- [ ] **§ 7's `transaction` over the SQLite arm**, including `{retries: n}` on `SQLITE_BUSY` — § 8
      maps it to `Deadlock`, which is one of the two kinds the retry re-runs on (0067 §§ 7, 8).
      `crates/nvs-stdlib/src/db.rs:4678` (`Transacting`, the enum needing a fifth arm),
      `crates/nvs-stdlib/src/db.rs:4762` (`transacting`, whose `other =>` is the last reader of
      `driverless`), `crates/nvs-db/src/sqlite.rs:590` (`begin`, and the accounting note at 660 on
      what a refused outermost `COMMIT` leaves behind).
- [ ] **A relative `[db.<name>] path` resolves against the config file's directory** (ADR 0103 § 5),
      which nothing does today. `crates/nvs-config/src/db.rs:72` (`crate::resolve::absolute`, the
      call `tls_ca_file` already makes), `crates/nvs-db/src/sqlite.rs:95` (the doc claiming it has
      already happened).

## Backlog

- `orient.py`'s map still does not print `crates/nvs-stdlib/src/db.rs`; the goal's `[context]
  modules` needs it, and every item above is inside that file.
- `Core\Db::stream` has no SQLite half and cannot have the same one — `nvs-db/src/sqlite.rs`'s module
  doc names the chunk size it would need.
- `queue`'s § 2 schema is written for three drivers; SQLite now binds and sends, so a SQLite
  migration is writable — `crates/nvs-stdlib/src/queue.rs:1731`.
- The five-driver CI matrix (`docs/plan/m8.md` verify) has no SQLite leg, which is the one that needs
  no container.
