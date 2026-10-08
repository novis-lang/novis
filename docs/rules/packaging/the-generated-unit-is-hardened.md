The generated unit carries what a hand-written one usually does not:

```ini
[Service]
Type=notify
ExecStart=/usr/bin/nvs serve --config /etc/nvs/nvs.toml
WatchdogSec=30
User=nvs-web
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=-/usr/bin/.nvsdata/cache -/usr/bin/.nvsdata/tmp -/usr/bin/.nvsdata/lsp -/usr/bin/.nvsdata/logs
PrivateTmp=true
CapabilityBoundingSet=
AmbientCapabilities=CAP_NET_BIND_SERVICE
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
SystemCallFilter=@system-service
```

**`ReadWritePaths=` is the write half of the install's grant list, and nothing else.**
`ProtectSystem=strict` leaves the service no writable path, so the unit names exactly the paths
`rule:packaging/a-service-runs-as-a-virtual-account` grants read/write — the data folder's `cache/`,
`tmp/`, `lsp/` and `logs/`, and a `file:` log directory, `[opcache] file_cache_dir` or `[io] temp_root`
outside them — derived from the same list a Windows install applies as ACLs. The data folder and its
`nvs.toml` stay read-only. Each path carries systemd's `-` prefix: a listed path missing at start fails
the whole unit with `226/NAMESPACE`, while a skipped one leaves the server running with the one warning
an unusable folder gets. A path is quoted where it has a space, a quote or a backslash, and a `%` is
doubled, since systemd expands specifiers in a path setting.

**`ProtectHome=true` unless the service needs a home directory.** `true` masks `/home`, `/root` and
`/run/user` and every path under them, a `ReadWritePaths=` entry included — systemd drops a path nested
under an inaccessible one. A unit whose binary, entry file or granted path is under one of them carries
`ProtectHome=read-only` instead, which leaves those directories readable as their file permissions allow
and lets `ReadWritePaths=` open the granted ones; every other unit keeps `true`. Refusing such an install
was the alternative, and it would refuse the common case of a binary in a user's home for no write the
service does not already hold.

`AmbientCapabilities` is emitted **only** when the configured `[server] listen` addresses include a
privileged port, so the ordinary case grants nothing at all. `MemoryMax` is derived from the config's
`[limits]` rather than invented. `Type=notify` means `READY=1` after the listener binds — so
`systemctl start` does not return before the port accepts — plus `RELOADING=1`/`STOPPING=1` at the
transitions and a `WATCHDOG=1` ping for as long as
`rule:http-server/a-wedged-core-is-detected-by-its-deadline`'s detector says a core is still turning.
**That ping is gated on the detector rather than written by an accept loop**, because a beat from a
thread that lives whether or not a core turns proves only that the process exists, which is not what
`WatchdogSec=` is asking. It is withheld when no core is turning at all, and never for one wedged core
of several — that one is shed (`rule:http-server/a-wedged-core-is-shed-never-killed`), and stopping the
whole process over it would end every healthy core's in-flight requests to answer one core's fault.
The `sd_notify` protocol is a datagram to
`$NOTIFY_SOCKET` and needs no `libsystemd`, so this adds no C dependency and
`rule:packaging/a-c-dependency-answers-two-questions` does not arise.

Socket activation — a privileged port with an empty capability set — is deferred rather than refused:
it changes how `nvs serve` acquires its listener, which makes it a server change the unit generator
would simply follow.
