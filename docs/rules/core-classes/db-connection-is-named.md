`Core\Db::connect` resolves a root-owned `[db.<name>]` block, so a credential never appears in
program source, never enters the repository, and is rotated without a deploy. `Core\Db::open` covers
what a name cannot — one database per tenant, chosen at request time — from a `Db\Settings` value.

Both memoize per request: `connect` keys on the *name*, `open` on a hash of *every* settings field.
Keying `connect` on the name keeps two identically-configured blocks as two connections, because an
operator who wrote two blocks meant two. Hashing all of `open`'s fields means a second call differing
only in `timeout` gets its own connection rather than silently inheriting the first caller's.
Hashing a `secret` password is safe, because a hash of a secret is neutral.
`{shared: false}` bypasses memoization, which is how a program writes an audit row that must survive
a rollback, holds session-scoped state — advisory locks, `SET LOCAL`, temporary tables — off the
shared connection, or writes while a stream occupies another. It bypasses memoization within the
request, not the pool across requests: a dedicated connection is still drawn from and returned to
its core's pool (`rule:security/db-pool-reset-is-a-boundary`).

`Db\Settings` is a discriminated union over enum-case types rather than one loose shape: SQLite takes
a `path` and has no `host`, so a `host` on a SQLite settings object is a compile error rather than a
silently ignored field.

A connection is released by the runtime at request teardown — the job a destructor would have done,
done by the arena instead. `close()` releases one early, and the name is then free for the next
`connect`.

**Declined:**
- **A DSN string**, `connect("pgsql:host=db;dbname=shop", $user, $pass)`. It is a mode string
  encoding structure the type system should carry (`rule:core-api/no-mode-strings`), and it puts
  credentials in source by default.
