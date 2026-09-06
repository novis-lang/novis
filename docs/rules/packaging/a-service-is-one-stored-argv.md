```
nvs service install   <name> [options] -- <verbatim nvs args…>
nvs service uninstall <name>
nvs service start | stop | status <name>
nvs service run       <name>                                  # the service manager's entry point
nvs service unit      <name> [options] -- <verbatim nvs args…>  # Linux: print, install nothing
```

A service is this binary, registered with the platform's service manager, running **one stored argv**.
Everything left of `--` belongs to the installer; everything right of it is stored untouched and never
interpreted, which is what makes every parameter `nvs` accepts passable. `--` is mandatory: without it,
`--start` is ambiguous between the installer and the hosted program, a defect `mysqld --install` has
and Novis does not inherit. `nvs install-service` is accepted as a hidden alias for the muscle memory
`mysqld --install` and `httpd -k install` built.

The verbs are namespaced like `nvs ctl` because they act on a server rather than on files, and the name
is positional and is the same identity `nvs ctl --socket` uses. `start`/`stop`/`status` are thin — the
SCM directly on Windows, `systemctl` by argv with no shell on Linux
(`rule:core-classes/process-is-argv-only`) — and earn their second spelling by reporting what no
service manager knows: the in-flight request count, and drain progress during a stop, asked over the
control socket (`rule:config/one-local-control-socket`).

What that argv may name is `rule:packaging/the-installer-is-a-sink`'s closed list.
