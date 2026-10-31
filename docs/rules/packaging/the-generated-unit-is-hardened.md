The generated unit carries what a hand-written one usually does not:

```ini
[Unit]
Requires=web.socket
After=network.target web.socket

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
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
SystemCallFilter=@system-service
```

and, for a server that binds a privileged port, `web.socket` beside it:

```ini
[Unit]
PartOf=web.service

[Socket]
ListenStream=0.0.0.0:443
ListenStream=127.0.0.1:8000

[Install]
WantedBy=sockets.target
```

**`ReadWritePaths=` is the write half of the install's grant list, and nothing else.**
`ProtectSystem=strict` leaves the service no writable path, so the unit names exactly the paths
`rule:packaging/a-service-runs-as-a-virtual-account` grants read/write — the data folder's `cache/`,
`tmp/`, `lsp/` and `logs/`, and a `file:` log directory, `[opcache] file_cache_dir` or `[io] temp_root`
outside them — derived from the same list a Windows install applies as ACLs. The data folder and its
`nvs.toml` stay read-only. Each path carries systemd's `-` prefix: a listed path missing at start fails
the whole unit with `226/NAMESPACE`, while a skipped one leaves the server running with the one warning
an unusable folder gets.

**A Unix socket in `[server] listen` gets its folder.** A socket's folder under `/run` (or `/var/run`)
is a `RuntimeDirectory=`: systemd creates it for the service account at every start, keeps it writable
under `ProtectSystem=strict`, and removes it at stop, so nothing is left over to make the next bind fail
and `/run` being emptied at boot costs nothing. `RuntimeDirectoryMode=` is the owner's `rwx` plus search
for the group and for others exactly where `socket_mode` lets them connect: `0750` for the default
`0660`, so a proxy added to the account's group reaches the socket and nobody else lists the folder. A
socket's folder anywhere else is the operator's, and the unit only adds it to `ReadWritePaths=`, never
changing its owner or mode. `/run` itself is never opened, because that would let the service write
every other service's runtime files; a socket placed directly in it needs a folder of its own. A folder
`RuntimeDirectory=` cannot name (a `.` or `..` segment, or a `:`, which it reads as a symlink) is opened
as written instead.

**No value is written into the unit raw.** Every `ExecStart` word, the binary included, and every
`ReadWritePaths=` path is quoted when it is empty, is a lone `;`, or has a space, a quote, a backslash or
a control character; inside the quotes `\` and `"` are escaped and a control character is written
`\xNN`. systemd decodes backslash escapes inside quotes and outside, ends a command at a bare `;`, and
ends a setting at a newline, so none of them can split a word or add a line. A `%` is doubled
everywhere, `Description=` included, because systemd expands specifiers in every word; a `$` is doubled
in an `ExecStart` argument, because systemd expands variables in the arguments and not in the binary.
An uninstall reads the binary and the argv back out of `ExecStart` through the exact inverse.

**`ProtectHome=true` unless the service needs a home directory.** `true` masks `/home`, `/root` and
`/run/user` and every path under them, a `ReadWritePaths=` entry included — systemd drops a path nested
under an inaccessible one. A unit whose binary, entry file or granted path is under one of them carries
`ProtectHome=read-only` instead, which leaves those directories readable as their file permissions allow
and lets `ReadWritePaths=` open the granted ones; every other unit keeps `true`. Refusing such an install
was the alternative, and it would refuse the common case of a binary in a user's home for no write the
service does not already hold.

**The socket unit is written only when the addresses the server binds include a privileged port**, and
without one the service unit is exactly as above minus its `Requires=` and the socket in `After=`.
**The addresses, and the sockets above, are the ones `serve` binds:** `[server] listen` of the
configuration the service reads (`127.0.0.1:8000` when it writes none), unless the stored argv gives
`serve` a `--listen`, which replaces the whole set with one TCP address, or a `--port`, which keeps each
entry's host and turns a socket into `127.0.0.1` on that port. Either flag therefore leaves no socket
folder. An argument of
`run` spelled `--port` is the program's and is not read. `MemoryMax` is derived from the config's
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

**A privileged port is bound by systemd, so the service never holds `CAP_NET_BIND_SERVICE`.** The
socket unit has one `ListenStream=` for each TCP address of the set, privileged or not, so a service's
TCP sockets have one way in; an address with port `0` is left to `nvs serve`, since a port systemd
chose would match no entry. A Unix socket stays `nvs serve`'s, bound under `RuntimeDirectory=` with
`socket_mode` on it. `install` writes both files, then enables the socket ahead of the service, and
`uninstall` disables the socket first, because while it listens a connection would start the service
again, then removes both. A restart keeps the socket open, so a connection made during it waits
rather than being refused.

**`nvs serve` adopts what systemd passes, on Linux.** At the top of the boot, before any thread, it
reads `LISTEN_PID` and `LISTEN_FDS` and removes both with `LISTEN_FDNAMES`. It takes the descriptors
from 3 only when `LISTEN_PID` is its own pid, so variables inherited from a parent take nothing, and
sets `FD_CLOEXEC` on each so no program it spawns inherits a listener. Each passed socket is the
listener of the TCP entry on its local address; an entry with none is bound as before. A passed
socket on an address no entry names stops the start, naming that address: `[server] listen` is
`Boot`-class, and under activation a changed set also needs `nvs service install` again, so that
refusal is the one place the mismatch can be seen. The protocol is two variables and a run of
descriptors, so it needs no `libsystemd` either. Elsewhere nothing is passed and nothing changes.

**Cost.** One descriptor per address, held for the life of the process; nothing per request. Declined:
`AmbientCapabilities=CAP_NET_BIND_SERVICE` for a privileged port, because a capability the
process holds is one a compromise of it holds too.
