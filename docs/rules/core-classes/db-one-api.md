`Core\Db` is the only way to reach a database. There is no procedural twin, no second object API, and
no escaping function: the query-text parameter refuses `tainted` while bound parameters accept it
freely (`rule:core-classes/db-parameters`), and an escaper would be a second, weaker answer to a
question already settled.

There is **no `prepare` step**. `query` and `execute` take SQL and parameters together, and each
connection holds an LRU cache of server-side prepared statements keyed by SQL text plus expansion
arity, sized by `statement_cache` in the connection's config block. Two spellings of one operation
would buy nothing the cache does not already provide.

The cost is recorded rather than hidden: on MySQL and MariaDB a statement's first execution in a
request costs two round trips and cached re-executions cost one, while PostgreSQL's extended protocol
pays nothing extra. Emulated prepares — string interpolation inside the driver — do not exist in any
form.

Permanently refused, and not deferred: multi-statement queries, which are the amplifier that turns
one injection into a compromise; `LOAD DATA LOCAL INFILE`; `PDO::quote` and
`mysqli_real_escape_string`; by-reference parameter or column binding; connection-level
`lastInsertId` state; `PDO::ATTR_*`; and `PDO::inTransaction`. Query builders, ORMs and migration
tooling are not `Core` at all.
