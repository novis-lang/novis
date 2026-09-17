```toml
[cache.shared]
url = "unix:/run/redis.sock"          # or redis://host[:port]

[db.main]
driver = "postgres"
host   = "/var/run/postgresql"        # a socket directory
```

Two spellings because the two keys are two different things, and each takes the one that reads as
itself. `[cache.shared] url` holds a URL and its parser already dispatches on scheme — it strips
`redis://` and `rediss://`, and refuses a credential written as userinfo and an index written as a
path, because `[cache.shared] password` and `[cache.shared] database` are the one place each of
those is written — so `unix:` is one more arm on machinery that exists, and the value stays a URL as
the key's name promises. A scheme names no protocol and does not need to: the block speaks RESP and
nothing else, so every one of the three says *which transport* and nothing more. That is also why
TLS is a scheme here and not a key beside the URL.

`[db.<name>] host` is not a URL and never was, so it takes the overload the listening side already
established for `[server] listen`: a value beginning with a path separator is a socket, and no
`host:port` can be spelled that way. What the path means per driver — a directory for Postgres, the
socket file for MySQL and MariaDB, a refusal for MSSQL — is
`rule:core-classes/db-unix-socket-path`'s.

A bare path in `url` is refused rather than read: a non-URL in a key called `url` is a thing to
re-litigate rather than a thing to read.
