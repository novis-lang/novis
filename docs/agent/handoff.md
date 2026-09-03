# Handoff

## State

**M8 goal 5, stage 8. SQLite is the fifth driver, and § 9's type map now runs on it end to end.**
`crates/nvs-stdlib/src/db.rs`'s `sqlite_column_value` keys a cell off the column's **declared** type
— a `date`, `datetime`, `time`, `uuid`, `decimal` or `boolean` column answers its `Core\Time`,
`Core\Uuid`, `decimal` or `bool` value, and a cell the declaration does not describe throws
(`unparsed_column`). That closes what the previous session had left as "known gap 9" and **reverses
its decision**: the deferral to `Core\Db\Row`'s typed reader is gone, and that module doc argues why
in full. § 9's own last sentence — "mapping keys off the *declared* column type and throws on a
value that does not parse" — is the natural-type rule, so `get`, `toArray`, all eleven typed
readers, `queryAs<T>` and `converted` read the value the decode built and none of them learns SQLite
exists. A request-driven reader would have loosened `->date()` on the four drivers with a real
`DATE` and could not have reached a `DATETIME` at all, § 6's eleven readers having no member for one.

**A `DECIMAL` column accepts a `REAL`, and that is affinity rather than a shortcut.** SQLite gives
`decimal(10,2)` NUMERIC affinity, so the `TEXT` `nvs_db::sqlite::encode` binds is already a `REAL`
by the time it is stored; refusing one on the way back would mean no `decimal` column on this
backend ever reads. `sqlite_column_value`'s doc argues it and says what is lost, and where.

**Two new seams carry the parsing, in the crates that own the classes**:
`crate::time::{date_of_text, time_of_day_of_text, datetime_of_text}` and `crate::uuid::of_text`,
each `None` for text that does not parse because the caller holds the column's name and the seam
does not. `jiff` reads SQLite's own `YYYY-MM-DD HH:MM:SS` separator, so no second grammar was
written.

**Still open on this driver: § 7.** `Transacting` has no SQLite arm, which is this module's known
gap 2 and the next item. `nvs_db::sqlite` has the commands.

**A `[db.<name>] path` is still not resolved against the config file's directory.** `nvs_config::db`
resolves `tls_ca_file` and nothing else, while `SqliteTarget::path`'s doc claims ADR 0103 § 5's
resolution "has already happened". One of the two is wrong and it is the tree.

The standing acceptance failure is still stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`,
`nvs_types::intrinsics`' known gap 6, and is not this goal's to close.

## Next group

**One file set: `crates/nvs-stdlib/src/db.rs`**, with `crates/nvs-db/src/sqlite.rs` read-only for
the first item and `crates/nvs-config/src/db.rs` for the third.

- [ ] **§ 7's `transaction` over the SQLite arm**, including `{retries: n}` on `SQLITE_BUSY` and
      `SAVEPOINT` nesting to any depth (0067 §§ 7, 8). `crates/nvs-stdlib/src/db.rs:4684`
      (`Transacting`, the enum with no SQLite arm), `crates/nvs-stdlib/src/db.rs:4768`
      (`transacting`, which refuses the driver today), `crates/nvs-stdlib/src/db.rs:4955`
      (`named_drivers`, the refusal's roster), `crates/nvs-db/src/sqlite.rs:82` (`SqliteTarget`, and
      the module's `begin`/`commit`/`roll_back` beside it).
- [ ] **A relative `[db.<name>] path` resolves against the config file's directory** (ADR 0103 § 5),
      which is one `origins` lookup beside the one `tls_ca_file` already does.
      `crates/nvs-config/src/db.rs:63` (the `tls_ca_file` loop this joins),
      `crates/nvs-db/src/sqlite.rs:82` (`SqliteTarget::path`, whose doc claims the resolution has
      already happened), `crates/nvs-stdlib/src/db.rs:3464` (`sqlite_settings`, the `open` arm,
      where a program-supplied path is a sink and stays relative to the process instead).
- [ ] **`Db\Row`'s eleven readers over a SQLite `BLOB` and a `time` column**, the two rows the new
      map reaches that no case asks about (0067 § 9). `crates/nvs-stdlib/src/db.rs:5574`
      (`sqlite_column_value`), `tests/conformance/core/db-a-sqlite-column-reads-back-as-the-type-its-schema-declared.nvst`.

## Backlog

- Narrowing may be weaker than it should be: `if ($a != null && $b != null)` does not narrow either
  receiver inside the body (`E0459`), while `if ($a != null)` does — worth a look against ADR 0066
  before a case works around it again. Playbook, *Writing a test case*.
- `[context] adrs` was missing **0067 § 6**, which this item's own text cites; it had to be sliced by
  hand. `docs/agent/loop-goal.toml`.
- `Core\Time\Date` has no `toString`, so a case renders one through `format("yyyy-MM-dd")` — ADR
  0063 R-numbering may or may not want one. `crates/nvs-stdlib/src/time.rs`.
- SQL Server is still `TdsConn`-stubbed for § 13's reset; `crates/nvs-db/src/conn.rs` owns it.
- `crate::queue`'s four members stay PostgreSQL-only. `crates/nvs-stdlib/src/queue.rs`.
