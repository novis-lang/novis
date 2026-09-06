# Handoff

## State

**Goal 20 — a configured store is authorized by its configuring — has just started; nothing of it has
landed yet.** Goal 19's whole list is this goal's Stage 1 floor, which is the parity program entire.

The design is settled and written:
[ADR 0142](../../adr/0142-a-configured-store-is-authorized-by-its-configuring.md), six sections, with
its cross-links already folded into `rule:security/net-address-policy`'s carve-out and `rule:core-api/two-cache-tiers`'s `shared()` row. **This
goal opens no ADR number** — a gap in 0142 is a folded edit to its body.

The short of it: `Core\Cache::shared()` and `Core\RateLimit::consume` stop asking `net.connect` at a
host and start asking `cache.shared`, unscoped, because the endpoint is one an operator wrote into
root-owned configuration and that writing is the authorization. Once the grant names the store instead
of an address, a store with no address is reachable — `unix:/run/redis.sock` in `[cache.shared] url`, a
bare path in `[db.<name>] host` — admitted only where an operator wrote it.

## Next group

**Stage 2: the grant** — one file set: `crates/nvs-config/src/capability.rs`,
`crates/nvs-stdlib/src/cache.rs`, `crates/nvs-stdlib/src/ratelimit.rs`,
`crates/nvs-stdlib/src/registry.rs`.

- [ ] **`Cap::CacheShared` on the roster** — `crates/nvs-config/src/capability.rs:34`, spelled
      `cache.shared`, asked at `Scope::Unscoped`. Its doc comment points at `Cap::MailSend`'s reasoning
      (`:54`) rather than restating it: the endpoint an operator wrote carries the authority that
      granted the capability, so it is pre-approved and is not asked about `rule:security/net-address-policy`'s ranges.
- [ ] **The door stops asking about an address** — `open_configured`
      (`crates/nvs-stdlib/src/cache.rs:@open_configured`) asks the new grant and drops `pin_host`, so
      neither caller walks the denied-range table. Both keep their own `remedy` clause.
- [ ] **The boot `Warn`** — a `[cache.shared] url` set with no `cache.shared` grant, named where both
      keys are visible. This is the migration path for a deployment that had granted `net.connect` for
      its store's host, and it is the only place that migration is reported.
- [ ] **Stage 0 in the same pass, because it is the same lines** — `SHARED_DOC`'s card and the
      `RuntimeError` description in `cache.rs`, the matching text in `ratelimit.rs`, the capability row
      in `registry.rs`, and any `nvs.toml` fixture under `tests/` still granting `net.connect` for a
      store's host. `docs/novis.md` is generated from those cards and is never edited by hand.

## Backlog

- **Stage 3 (the cache transport)** shares `cache.rs` with stage 2, so a session that lands stage 2 with
  headroom takes it: the `unix:` arm in `endpoint`, `E0627` beside `E0626` in
  `crates/nvs-diagnostics/src/lib.rs`, and widening `open_shared`'s per-core key from a `SocketAddr` to
  an address-or-path — that key is what decides reuse-or-replace, which is how a reloaded configuration
  looks from there. `NvsUnix` already exists (`crates/nvs-host/src/net.rs:446`) and needs nothing.
- **Stage 4 (the driver transport)** shares no files with either and is its own group:
  `crates/nvs-db/src/{conn,mysql,maria,pg,tds}.rs` plus the matrix's socket leg. Postgres derives
  `<host>/.s.PGSQL.<port>`; MySQL and MariaDB open the path as written; TDS refuses one.
- When this goal's last check goes green the driver takes goal 50 — the dossier.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
