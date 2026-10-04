A SQL argument given as a string literal is validated while checking, under the closed list of
intrinsic calls (`rule:expressions/intrinsic-list-is-closed`): placeholder count against a params
array literal, positional-versus-named consistency, an unterminated string literal, and a refused
second statement. A `Db::open` host given as a string literal and matching no
`db.open` grant is likewise a check-time diagnostic, since configuration is read at boot on the
machine that compiles.

Full per-dialect SQL parsing is **not** done, and never will be: it would mean maintaining four
vendors' grammars in the front end forever. That refusal covers DDL as well as queries, which is why
`rule:core-classes/schema-introspection` reads an existing database by catalog query rather than by
parsing the `CREATE TABLE` a server prints.

What this costs is that a column name typo survives to run time. What it buys is that the compiler
never has to be right about a dialect it does not own.
