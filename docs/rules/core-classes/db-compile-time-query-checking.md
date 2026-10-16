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
never has to be right about a dialect it does not own. The host check reads the grant through the
same predicate a request does, so the two cannot disagree about a pattern.

**Declined:**
- **Schema-aware checking now**, validating literal queries and `queryAs<T>` shapes against a live
  schema. It is the most valuable idea here, and it needs a reachable database at check time, so it
  can only be a developer-machine mode; a schema read off a live server into a file
  (`rule:core-classes/schema-introspection`) is the cache it would read.
