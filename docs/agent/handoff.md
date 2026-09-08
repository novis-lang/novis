# Handoff

## State

**Goal 20's Stage 2 — the keystone — is landed whole, with Stage 0's catch-up in the same pass.**
`Cap::CacheShared` is on the roster (`cache.shared`, `Scope::Unscoped`), `open_configured` asks it
after reading the directive and dials the store with `nvs_runtime::capability::resolve_host` instead
of `pin_host`, and neither `Core\Cache::shared()` nor `Core\RateLimit::consume` walks
`rule:security/net-address-policy`'s table any more.
`rule:config/cache-shared-is-the-grant-over-the-configured-store` is `shipped` and its "Not
shipped" paragraph is gone.

The migration is reported at boot: `crates/nvs-config/src/store.rs` is a new module holding
`W1008` — a `[cache.shared] url` with no `cache.shared` grant — hung off `resolve` beside
`session::validate`. It is the home for the `AF_UNIX` boot refusal Stage 3 owes as well.

`nvs.toml`'s `examples/cache.nvs` block now grants `cache.shared` alone; its `net.connect` /
`net.internal` pair is gone, which is the whole point of the rule.

Nothing is blocked. The five Stage 2 acceptance tests exist and pass:
`a_configured_store_needs_no_address_grant`,
`a_configured_store_is_refused_without_its_own_grant`,
`a_loopback_store_needs_no_net_internal_exception`,
`the_limiter_and_the_tier_ask_one_grant_at_one_door` (all in `crates/nvs-stdlib/src/cache.rs`) and
`a_configured_url_without_its_grant_is_a_boot_warning` (`crates/nvs-config/src/store.rs`).

## Next group

**Stage 3: the transport for the cache tier** — one file set: `crates/nvs-stdlib/src/cache.rs`,
`crates/nvs-stdlib/src/cache/redis.rs`, `crates/nvs-diagnostics/src/lib.rs`.

- [ ] **`unix:` in `[cache.shared] url`** — `crates/nvs-stdlib/src/cache.rs:614`, one arm beside the
      `redis://` one, per `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`. The
      `rediss://`, path-as-database-index and bad-port refusals are untouched, and a program-supplied
      path is still refused at every other door
      (`rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`).
- [ ] **The per-core key widens from a `SocketAddr` to an address-or-path** —
      `crates/nvs-stdlib/src/cache.rs:663` (`open_shared`) and `:571` (`SHARED`). That key is what
      decides reuse-or-replace, which is how a reloaded configuration looks from there.
      `NvsUnix` already exists at `crates/nvs-host/src/net.rs:486` and needs nothing.
- [ ] **`E0627` beside `E0626`** — `crates/nvs-diagnostics/src/lib.rs:1637`, a Unix spelling on a
      build with no `AF_UNIX` transport refused at boot with a note naming the platform,
      `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot`. The refusal is a
      refusal and never a fallback to loopback TCP; the check belongs in
      `crates/nvs-config/src/store.rs`, which this goal created for exactly that neighbourhood.
- [ ] **The proofs** — a record written through the socket and read back through it against a
      `UnixListener` the way `crates/nvs-stdlib/src/cache/redis.rs:573` drives a `TcpListener`, and
      the boot refusal asserted by the diagnostic rather than by a failed connect.

## Backlog

- **Stage 4 (the driver transport)** shares no files with Stage 3 and is its own group:
  `crates/nvs-db/src/{conn,mysql,maria,pg,tds}.rs` plus the matrix's socket leg. Postgres derives
  `<host>/.s.PGSQL.<port>`; MySQL and MariaDB open the path as written; TDS refuses one.
  `crates/nvs-config/src/db.rs`'s `is_relative_file` is the neighbourhood for reading a path in
  `[db.<name>] host`.
- `docs/agent/goals/4-core-part-ii.md:135` still says the shared tier is "gated by `net.connect`" —
  closed-goal prose, left alone on purpose; `docs/agent/goals/README.md` owns whether those are
  edited.
- When this goal's last check goes green the driver takes goal 50 — the dossier.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
