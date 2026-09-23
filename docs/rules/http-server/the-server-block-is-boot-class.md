```toml
[server]
root               = "/www"
listen             = ["127.0.0.1:8000"]   # restart required; "host:port", or an absolute path meaning a Unix socket
socket_mode        = "0660"               # restart required; Unix-socket entries only
dispatch           = "entry"              # a mode-selected startup default; development "path"
static             = false                # a mode-selected startup default; development true
trusted_proxies    = []                   # fail-closed
health_path        = ""                   # off
max_in_flight      = 10000
workers            = 4                    # restart required; accept cores; unwritten is this machine's parallelism
header_timeout     = "10s"
body_idle_timeout  = "30s"
write_idle_timeout = "30s"
keepalive_timeout  = "75s"
drain_timeout      = "30s"                # how long a connection is served after it sees the drain
```

The whole block is `System`-class (`rule:config/three-changeability-classes`): `header_timeout` and `keepalive_timeout` apply before any Novis code exists on a connection, so a `Runtime` class would be a promise the block could not keep. **Three keys need a restart: `listen`, `socket_mode` and `workers`** (`rule:config/reloadability-is-its-own-field`). Every other key applies without one. `dispatch`, `static`, `trusted_proxies`, `health_path` and `max_in_flight` are read from the snapshot a request cloned, so a reload reaches the next request. The four waits, `drain_timeout` and `[server.connection]` are read from the snapshot published when a connection is accepted, so a reload reaches the next connection, and a connection already open keeps what it was accepted under. `root` and `[[server.mount]]` are read from the snapshot a reload published, and a change to either expands the mount table again (`rule:http-server/a-mount-table-expands-at-boot`).

`listen` is one flat array — an entry beginning with a separator is a Unix socket (`rule:http-server/a-unix-socket-listener`), and no `host:port` can be spelled that way. **The default is `127.0.0.1:8000` in both modes**: loopback is the proxied shape as well as the development one, so demanding an explicit `listen` in production would be friction with no safety in it. `nvs serve --listen`/`--port` overrides the file, on the same precedent that makes the mode flag the last word (`rule:config/the-mode-flag-wins-over-the-file`); `--port` alone keeps the host the file chose.

`workers` is the one key here the machine answers a default for, and it is the only one that says how many accept loops there are rather than what one of them does: `rule:http-server/the-accept-fan-out-is-one-worker-per-core` owns the count, what a core holds of its own and what every core shares.

The accept loop backs off on descriptor exhaustion and logs once per window rather than once per attempt, and a core that stops making progress is reported and shed by a watchdog reading the in-flight deadline each worker already keeps. The waits, the valve, the probe and the drain period are their own rules: `rule:http-server/four-idle-waits-all-finite`, `rule:http-server/max-in-flight-refuses-before-allocating`, `rule:http-server/health-path-is-off-and-checks-nothing`, `rule:concurrency/a-drain-closes-a-connection-cleanly`. `drain_timeout` is the one key here that bounds a connection which is working rather than an idle one, which is why it is not `keepalive_timeout` under another name: an operator lengthening the keep-alive a proxy needs would otherwise have lengthened every restart.
