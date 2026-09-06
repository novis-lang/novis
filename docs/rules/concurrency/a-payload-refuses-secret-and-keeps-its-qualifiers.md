A durable row is an output, and an output refuses `secret` (`rule:security/secret-qualifier`). So a
`secret` cannot enter a job payload — the enqueue does not compile. A job that needs a credential
reads it from configuration when it runs, which is where credentials live anyway and which keeps them
out of a table, a backup and a replica.

A payload's other qualifiers are recorded with it and restored on decode: a `tainted` value enqueued
comes back `tainted` (`rule:security/tainted-qualifier`), so the analysis survives the round trip
instead of being laundered by a database. The queue's table is trusted exactly as far as the rest of
the application's database is — an attacker who can write to it has already won — which is a better
answer than returning every field `tainted` and training every job to launder reflexively.
