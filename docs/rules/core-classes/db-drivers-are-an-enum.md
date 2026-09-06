The five drivers are one enum — PostgreSQL, MySQL, MariaDB, SQL Server, SQLite — each variant owning
its own state machine, error-code table and reset. `Core\Db`'s entry points `match` once.

Three reasons, in priority order. **The set is closed**: a new backend is a decision record, not a
plugin, and a sandboxed extension cannot host one anyway, since a pool is the thing a sandbox
boundary cannot hold — open-set extensibility is the one property a trait buys and this design does
not want it. **A trait wide enough for all five would be half unimplemented**: `executeMany` is one
prepare and N executions on four drivers and a bulk protocol message on the fifth, a prepare is a
round trip on two and free on one, and a reset is four command sequences and a rollback on the fifth.
A trait method four drivers implement by returning an error is a lie the type system helped tell.
**Static dispatch on the request path** costs no allocation and no indirect call.

Where a signature repeats five times, it repeats. What keeps the five honest is not a type but one
assertion set run against five real servers. What is shared is the half with no driver in it — the
placeholder rewriter, the statement cache, the pool, the Novis side of the type map, and error-kind
normalisation, whose per-driver code tables are *data* each driver supplies rather than behaviour it
overrides.
