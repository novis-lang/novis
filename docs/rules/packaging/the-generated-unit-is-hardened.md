The generated unit carries what a hand-written one usually does not:

```ini
[Service]
Type=notify
ExecStart=/usr/bin/nvs serve --config /etc/nvs/nvs.toml
ExecReload=/usr/bin/nvs ctl reload --socket /run/nvs/control.sock
WatchdogSec=30
User=nvs-web
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
CapabilityBoundingSet=
AmbientCapabilities=CAP_NET_BIND_SERVICE
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
SystemCallFilter=@system-service
```

`AmbientCapabilities` is emitted **only** when the configured `[server] listen` addresses include a
privileged port, so the ordinary case grants nothing at all. `MemoryMax` is derived from the config's
`[limits]` rather than invented. `Type=notify` means `READY=1` after the listener binds — so
`systemctl start` does not return before the port accepts — plus `RELOADING=1`/`STOPPING=1` at the
transitions and a watchdog ping from the accept loop. The `sd_notify` protocol is a datagram to
`$NOTIFY_SOCKET` and needs no `libsystemd`, so this adds no C dependency and
`rule:packaging/a-c-dependency-answers-two-questions` does not arise.

Socket activation — a privileged port with an empty capability set — is deferred rather than refused:
it changes how `nvs serve` acquires its listener, which makes it a server change the unit generator
would simply follow.
