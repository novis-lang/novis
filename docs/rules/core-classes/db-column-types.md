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
as reflected. The same table is read the other way by `rule:core-classes/schema-vocabulary-is-closed`.
