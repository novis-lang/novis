Sets the network store that `Core\Cache::shared` uses, where every server process and every machine sees
the same values.

`url` is the address of the store. It has no default. A server that sets no `url` has no shared cache. The
other keys say how the server connects. `password` is the password, or `password_file` names a file that
contains it. A container often passes a password in that way. `database` is the database in the store
that the values use. `timeout` limits the connect and each command, so a store that stops responding
cannot block a request.

A program also needs the capability `cache.shared` before it can use the store.

**Good to know:** you can change this block while the server runs. The next request uses the new store,
and so does the next scheduled job with `scope = "fleet"`. You do not need to restart the server.
