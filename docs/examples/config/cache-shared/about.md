Names the store the shared cache tier lives in: the one place a whole deployment reads the same entries
from.

`url` is the only setting here without a sensible default, and a deployment that writes none has no shared
tier at all. The rest is how that store is reached: `password`, or `password_file` naming a file whose whole
content is the value, which is how a container hands a credential in; `database`, for which of the store's
databases these entries live in; and `timeout`, which bounds both the connect and each command, so a store
that has stopped answering cannot hold a request open. Naming a store is only half of it — an application
also has to be granted the shared cache before it may open one.

**Good to know:** you can change this block while the server runs. The next request uses the new store,
and so does the next scheduled job with `scope = "fleet"`. You do not need to restart the server.
