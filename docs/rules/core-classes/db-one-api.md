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

A dynamic identifier, a table or column name SQL cannot bind, goes through `Core\Db::quoteIdentifier`
or an allowlist check.

**Declined:**
- **Mirroring PDO**: `prepare`/`execute`, fetch-mode constants, `quote()`, `lastInsertId()`,
  `errorInfo()`. It is the most familiar shape and the easiest conversion target, and each piece breaks
  a `Core` rule: the flag constants are mode arguments (`rule:core-api/no-mode-strings`), the error
  accessors are ambient state where a throw belongs (`rule:core-api/failure-throws`), the binding is
  by reference, the prepare/query pair is two names for one operation
  (`rule:core-api/one-paradigm-per-operation`), and the escaper is the weaker answer above.
- **Drivers as sandboxed extensions.** A connection pool is state that outlives a request, which is
  the one thing a sandbox boundary cannot hold (`rule:core-classes/db-drivers-are-an-enum`).
- **A query text that must be a literal in the source.** It would block legitimate dynamic assembly,
  such as pagination or an allowlisted sort column, to buy a guarantee the untainted rule does not
  need. If real programs show untainted text too easy to build without validation, this is the
  stricter mode to reconsider.
