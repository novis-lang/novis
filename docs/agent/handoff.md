# Handoff

## State

**Goal `outbound-proxy` is functionally complete.** `python tools/loop.py --goal-only` was 772 of 773
green: every stage 2–5 check passes, both `[http.client.proxy]` rules render `shipped`, and the one
red check — the carried socket floor — is answered in `tests/db/compose.yaml`, below.

The wip commit the driver swept up after the last session — `e5f5656e5`, unverified when it was made
— is verified now and is the whole of stage 4's launderer half: `crates/nvs-stdlib/src/http.rs:405`
(`pin`) is the one door `allowUrl` and every call take, so `resolve = "proxy"` does not leave
`tainted` on every URL in the network the word exists for; an approved set with nothing in it is the
approval of a host only a proxy resolves, told apart from a slot another writer mangled at
`crates/nvs-stdlib/src/http.rs:2714` (`addresses_of`) and refused at
`crates/nvs-stdlib/src/http/transport.rs:1462` where nothing tunnels it.

**The socket floor was the compose file's, not the tree's or the operator's.** `wsl
examples/cache-shared-socket.nvs [1 floor]` wants the socket `tests/db/compose.yaml`'s `redis`
service binds, and the source it named, `/mnt/wsl/novis-redis`, resolved in the daemon distro's own
private `/mnt/wsl`. The tmpfs every distro shares is mounted there at `/mnt/host/wsl`, so the source
is now `${NOVIS_SOCKET_DIR:-/mnt/host/wsl/novis-redis}` and the leg finds the socket at the path
`examples/cache-shared-socket.toml` spells; run by hand on the WSL leg the fixture prints both lines
the check wants. The `warning[W1008]` line it also prints is real and is the next group.

## Next group

**The socket fixture's ungranted store** — one file set: `crates/nvs-config/src/store.rs`,
`examples/cache-shared-socket.toml`.

- [ ] **W1008 is asked of the top-level `[capabilities]` block alone** —
      `crates/nvs-config/src/store.rs:178` (`advise`) reads `config.capabilities`, so
      `examples/cache-shared-socket.toml:22`'s `[app.capabilities.cache] shared = true` earns the
      warning on every run of the fixture. Decide whether an app-scoped grant answers the census —
      `rule:config/cache-shared-is-the-grant-over-the-configured-store` — and fix whichever side is
      wrong: the warning fires today over a tree that does grant the store.
- [ ] **Whether that `[[app]]` block applies to this run at all** —
      `examples/cache-shared-socket.toml:18` spells `entry = "cache-shared-socket.nvs"` while every
      block in the repository's own root `nvs.toml` spells a repo-rooted path, and the fixture is given
      to the CLI as `examples/cache-shared-socket.nvs`. Which of the two the matcher compares is what
      decides whether the grant above it is ever read, and it is **not checked**.

## Backlog

- `crates/nvs-stdlib/src/http/transport.rs:1771` answers one `IOError` naming every destination the
  proxy refused; no case covers a set of more than one where *all* of them are refused.
- The environment is never read for a proxy — a session that finds `HTTP_PROXY` on its path writes it
  here, per `docs/agent/loop-goal.md` § *Standing decisions*.
