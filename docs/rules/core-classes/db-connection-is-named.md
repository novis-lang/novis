`Core\Db::connect` resolves a root-owned `[db.<name>]` block, so a credential never appears in
program source, never enters the repository, and is rotated without a deploy. `Core\Db::open` covers
what a name cannot — one database per tenant, chosen at request time — from a `Db\Settings` value.

Both memoize per request: `connect` keys on the *name*, `open` on a hash of *every* settings field.
Keying `connect` on the name keeps two identically-configured blocks as two connections, because an
operator who wrote two blocks meant two. Hashing all of `open`'s fields means a second call differing
only in `timeout` gets its own connection rather than silently inheriting the first caller's.
`{shared: false}` bypasses memoization, which is how a program writes an audit row that must survive
a rollback or holds session-scoped state off the shared connection.

`Db\Settings` is a discriminated union over enum-case types rather than one loose shape: SQLite takes
a `path` and has no `host`, so a `host` on a SQLite settings object is a compile error rather than a
silently ignored field.

A connection is released by the runtime at request teardown — the job a destructor would have done,
done by the arena instead. `close()` releases one early, and the name is then free for the next
`connect`.
