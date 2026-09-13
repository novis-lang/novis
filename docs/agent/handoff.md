# Handoff

## State

**Goal `outbound-proxy` is met.** Every stage 2–5 check was already green and the one carried red —
`wsl examples/cache-shared-socket.nvs [1 floor]` — passes now: built into
`/var/tmp/nvs-target-wsl` and run on the WSL leg, the fixture prints both `want` lines, exits 0, and
prints no `warning[W1008]`.

**`W1008`'s census is over the whole tree, not the global block alone.**
`crates/nvs-config/src/store.rs` (`reachable`) folds every `[[app]]`'s own `[app.capabilities]` in
beside `[capabilities]`, because an app block folds over the global tree for the entry files it
matches and one grant anywhere is an application that reaches the tier —
`rule:config/cache-shared-is-the-grant-over-the-configured-store` carries that sentence now.

**The `[[app]]` block in `examples/cache-shared-socket.toml:17` does apply to this run.**
`crates/nvs-config/src/app.rs:83` resolves a block's key against the directory of the *file it was
written in*, so `entry = "cache-shared-socket.nvs"` is `examples/cache-shared-socket.nvs`; the
fixture reaching the store over the socket is the run-time half of the same answer, since nothing
else in that tree grants `cache.shared`.

The redis socket is on disk at `/mnt/wsl/novis-redis/redis.sock` — the bind source
`tests/db/compose.yaml` names, `/mnt/host/wsl/novis-redis`, is the *daemon's* spelling of the tmpfs
every distro sees at `/mnt/wsl`.

## Next group

**The proxy's refusal paths** — one file set: `crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **No case covers a proxy refusing every address in an approved set of more than one** —
      `crates/nvs-stdlib/src/http/transport.rs:1771` joins one `IOError` naming each address and the
      status it answered, and `a_proxy_refusing_connect_is_an_io_error_and_407_names_authentication`
      at `crates/nvs-stdlib/src/http/transport.rs:3044` walks a set of one, so the join and the
      "every destination it was asked for" wording are unasserted.
      `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise` is the rule
      that makes the set more than one — one `CONNECT` per approved address.
- [ ] **The `407` arm stops the walk, and nothing says so** —
      `crates/nvs-stdlib/src/http/transport.rs:1762` returns on the first `407` rather than asking
      the rest of the set, which is the behaviour
      `rule:http-server/an-outbound-proxy-is-operator-configured` describes as a refusal of the
      tunnel and not an answer from the destination; the same case can assert it with a two-address
      set whose first address answers `407`.

## Backlog

- The environment is never read for a proxy — a session that finds `HTTP_PROXY` on its path writes
  it here, per `docs/agent/loop-goal.md` § *Standing decisions*.
- `website/src/data/rules.json` and the pages under `website/src/content/docs/docs/rules/` are
  synced as of this session; nothing in `verify.py` watches them (`docs/agent/playbook.md`, the
  `sync:rules` bullet).
