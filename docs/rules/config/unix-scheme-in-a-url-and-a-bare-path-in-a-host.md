```toml
[cache.shared]
url = "unix:/run/redis.sock"          # or redis://host[:port]

[db.main]
driver = "postgres"
host   = "/var/run/postgresql"        # a socket directory
```

Two spellings because the two keys are two different things, and each takes the one that reads as
itself. `[cache.shared] url` holds a URL and its parser already dispatches on scheme — it strips
`redis://`, refuses `rediss://` with a sentence, and refuses an index written as a path because
`[cache.shared] database` is the one place an index is written — so `unix:` is one more arm on
machinery that exists, and the value stays a URL as the key's name promises. The scheme
names no protocol and does not need to: the block speaks RESP and nothing else, so its whole job is to
say *which transport*.

`[db.<name>] host` is not a URL and never was, so it takes the overload the listening side already
established for `[server] listen`: a value beginning with a path separator is a socket, and no
`host:port` can be spelled that way. What the path means per driver — a directory for Postgres, the
socket file for MySQL and MariaDB, a refusal for MSSQL — is
`rule:core-classes/db-unix-socket-path`'s.

A bare path in `url` is refused rather than read: a non-URL in a key called `url` is a thing to
re-litigate rather than a thing to read.
