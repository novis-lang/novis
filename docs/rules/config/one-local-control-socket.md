```toml
[control]
socket = "/run/nvs/control.sock"   # \\.\pipe\nvs-control on Windows; `false` disables
```

**Local socket only. There is no TCP listener, no token, no TLS and no auth middleware** — the
socket's owner and mode are the authentication. It is created mode `0600` (a DACL naming this account
on Windows), owned by the runtime's account, and **the server refuses to start if the directory
holding it is writable by any other account**, the same trust check every configuration file gets.
A tree that writes no `[control]` block gets no control surface at all. A reload that changes
`socket` creates the new endpoint, under the same directory check, before the old one stops
answering, and a reload pushed over the old one is answered there; `false` closes it. A new
endpoint that cannot be created is logged by name with the reason, and the running one stays and
is named as not applied. The socket exists only where
a long-running server does; `nvs run` compiles one file and exits.

The wire protocol is HTTP over that socket, not a bespoke line protocol: `curl --unix-socket` debugs
it with no special tooling. **`nvs ctl` is the client**, a namespace of its own because every other
subcommand acts on files with no server involved; `--socket` addresses one of several servers on a
host. `reload` re-reads the whole configuration tree and publishes it; `ctl config` prints the live
snapshot with each directive's origin. Operations serialize, so two reloads cannot interleave two
snapshots. **No control operation runs user Novis code, ever** — one that could would be
`rule:security/no-eval`'s door with a different name on it.

The wire shape is unstable until 1.0: every response carries the server version, and `nvs ctl`
refuses a mismatch. Every reload is written to `Core\Log` with its outcome.
