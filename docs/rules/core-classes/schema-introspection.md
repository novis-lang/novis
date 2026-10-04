Reading an existing database is one catalog reader per driver, answering the same schema value a
builder produces: `information_schema` for MySQL, MariaDB and SQL Server, `pg_catalog` for
PostgreSQL, and `sqlite_master` with the table and index pragmas for SQLite.

There is **no DDL parser, at any tier, in any form** — not "just for the CLI", not "best-effort", not
"just for `CREATE TABLE`". `rule:core-classes/db-compile-time-query-checking` already refuses to maintain
four vendors' query grammars, and that reasoning does not weaken for DDL, where the dialects diverge
more rather than less.

Introspection is also the *better* answer, not merely the cheaper one: a catalog is structured tables
rather than a language, the `CREATE TABLE` a server prints came from that server's own catalog
anyway, and the introspector must exist for the diff regardless — so a parser would be a second,
worse implementation of a job already done. Offline `.sql` input, if it is ever wanted, is a separate
tool with its own decision.
