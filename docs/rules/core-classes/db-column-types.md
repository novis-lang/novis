Every column has one natural Novis type, and `get()` and `toArray()` answer it. Asking for another
type converts when that is lossless and throws `DbError` otherwise: MySQL's `TINYINT(1)` is naturally
`int` and reads as `bool` on request with a stored `7` throwing; a `BIGINT UNSIGNED` past `i64::MAX`
reads as `uint` and throws for `int`; a `DECIMAL` refuses a `float` field. One rule settles what would
otherwise be a list of special cases.

The map itself is fixed: integers to `int` or `uint`, `DECIMAL`/`NUMERIC`/`MONEY` to `decimal`, text
families to `tainted string`, binary families to `tainted bytes`, `DATE`/`TIME`/`TIMESTAMPTZ`/`UUID`
to their `Core` types, PostgreSQL arrays to `array<T>`, and everything with no Novis type — `inet`,
ranges, `hstore`, geometry, `interval` — to `tainted string`. **JSON is not auto-decoded**, because
MariaDB's `JSON` is a `LONGTEXT` alias and so is not reliably detectable from metadata at all. A
zone-less `DATETIME` reads in a zone the connection declares, sent to the server as a numeric offset
so `CURRENT_TIMESTAMP` agrees.

There is deliberately **no universal string**: the universal path is `get(): mixed` plus the
language's own `as`, so `Core\Db` never grows a second stringification table. Every value a row
yields is `tainted` where its type can carry it, which closes stored injection by the same mechanism
as reflected. Only `string` and `bytes` carry the qualifier, so arithmetic on an `int`, `decimal` or
`Instant` column needs no laundering.

`query` yields a dynamic `Db\Row` whose typed readers answer `?T`, a NULL column being an ordinary
absence; `queryAs<T>` hydrates an inline shape or a `Core\Db\Codec` class, the same split as
`Json::decode` and `decodeAs<T>`. Field names match column names exactly, so there is no
snake-to-camel layer; `AS` in the SQL, or a derived codec's per-field `name`
(`rule:core-classes/derive-field-list`), is the way to rename. SQLite has no date or time types: its mapping
keys off the *declared* column type and throws on a value that does not parse. The declared zone is
a property of one database, written by the operator who knows what its columns mean, not an ambient
process or request default (`rule:core-api/no-ambient-state`).

The same table is read the other way by `rule:core-classes/schema-vocabulary-is-closed`: one table
used twice rather than two to keep in step.

**Declined:**
- **`string` as a universal column reader.** It ports PHP code almost unchanged, and it needs a second
  conversion table and makes the stringly-typed style frictionless; PHP's everything-is-a-string is an
  artefact of the MySQL text protocol.
- **Decoding JSON columns automatically.** It would work on three drivers and not on MariaDB, and
  `Core\Json::decode` is one call.
- **A `Zone` argument at every read.** `queryAs<T>` has no call site to pass it at.
- **`interval` as `Duration`.** An interval carries months, which a duration cannot.
- **A named zone sent to the server.** Named zones need MySQL's zone tables populated, which they
  usually are not, so the zone is sent as a numeric offset.
