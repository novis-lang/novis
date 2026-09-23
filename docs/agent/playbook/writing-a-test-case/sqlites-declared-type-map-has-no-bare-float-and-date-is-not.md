- **SQLite's declared-type map has no bare `float`, and `date` is not a conversion target at all.**
  `nvs_db::catalog::sqlite_scalar` reads `real`, `double precision` and `double` into the float
  family and nothing else, and `rule:types/conversion` has no `date` row, so `float not null` in a
  `create table` and `"2026-03-01" as date` in a parameter list both fail in a case whose `int` and
  `decimal` columns read fine. Read `crates/nvs-db/src/catalog.rs:598` for the driver's own
  spellings before declaring a table, and pass a day as the text the driver parses.
  [until: gone crates/nvs-db/src/catalog.rs:sqlite_scalar]
