# Handoff

## State

**Goal `outbound-proxy` is functionally complete.** `python tools/loop.py --goal-only` is 772 of 773
green: every stage 2–5 check passes, both `[http.client.proxy]` rules render `shipped`, and the one
red check is a carried floor one that no change to this tree closes.

The wip commit the driver swept up after the last session — `e5f5656e5`, unverified when it was made
— is verified now and is the whole of stage 4's launderer half: `crates/nvs-stdlib/src/http.rs:405`
(`pin`) is the one door `allowUrl` and every call take, so `resolve = "proxy"` does not leave
`tainted` on every URL in the network the word exists for; an approved set with nothing in it is the
approval of a host only a proxy resolves, told apart from a slot another writer mangled at
`crates/nvs-stdlib/src/http.rs:2714` (`addresses_of`) and refused at
`crates/nvs-stdlib/src/http/transport.rs:1462` where nothing tunnels it.

**The red check is the operator's, not the tree's.** `wsl examples/cache-shared-socket.nvs [1 floor]`
wants the socket `tests/db/compose.yaml`'s `redis` service binds at `/mnt/wsl/novis-redis`. The
service is up and healthy and holds the socket, but `wsl.exe -d docker-desktop -- ls /mnt/wsl` shows
that directory in the daemon distro alone while the leg's own `/mnt/wsl` never has it — `docker` is
also absent from the leg distro's PATH. The `warning[W1008]` line the ledger reports is not the
failure; the fixture exits 0 and prints an `IOError` for the missing socket. The plan's `Blocking`
field names the choice.

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

- A second `needs` for the socket fixture — `tools/loop.py:1568` (`LEG_NEEDS`) holds one entry and
  its comment invites the second; it waits on the decision the plan's `Blocking` now names.
- `crates/nvs-stdlib/src/http/transport.rs:1771` answers one `IOError` naming every destination the
  proxy refused; no case covers a set of more than one where *all* of them are refused.
- The environment is never read for a proxy — a session that finds `HTTP_PROXY` on its path writes it
  here, per `docs/agent/loop-goal.md` § *Standing decisions*.
