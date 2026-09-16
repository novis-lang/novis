# Handoff

## State

**Goal `unowned-closures`, stage 5: the first of the stage's two checks is closed.** A deployment now
writes `[server.connection]` — `max_open`, `max_frame`, `max_message`, `idle_timeout`, `max_lifetime`,
`send_timeout` — and every key it leaves out keeps the number `nvs_server::bounds::Connection::default`
ships. `nvs_config::server::connection_bounds_for` (`crates/nvs-config/src/server.rs:317`) owns the keys
and the refusals: `false` and zero are both `E0649` in any of the three units, taken in `validate` before
a listener exists. It hands back *overrides* and holds no default of its own, which is what keeps the
bounds one crate's numbers; `Connection::configured` (`crates/nvs-server/src/bounds.rs:206`) applies them
and raises a written `max_message` to the frame bound. The table rides to every connection on `Serving`
(`crates/nvs-server/src/serve.rs:575`, filled at boot by `Serving::bounded_by` from
`crates/nvs-cli/src/serve.rs:206`), because a bound on what a request leaves behind is the server's and
not one connection's. `drain`, `subscriber_queue` and `reconnect` have no key on purpose, stated in that
module's header. Nothing is blocked.

## Next group

**Stage 5: the compiled-unit cache key** — one file set: `crates/nvs-config/src/cache.rs` and
`crates/nvs-cli/src/cache.rs`, the two halves of one digest.

- [ ] **A rebuilt compiler is a new cache key** — `crates/nvs-config/src/cache.rs:140`,
      `rule:config/the-extension-set-is-in-every-unit-key`. `env_hash` folds `CARGO_PKG_VERSION` and
      `debug_assertions` where `docs/decisions/0078.md:161` spells a `compiler_version_hash`, so two
      builds of one unreleased version key their units the same. The check wants
      `two_builds_of_one_release_version_key_their_units_apart`. **Settle the stamp's shape first**: a
      `build.rs` that hashes every crate's sources makes any edit anywhere rebuild `nvs-config` and so
      the whole workspace, which is the dev loop's own cost — so either stamp only when `PROFILE` is
      `release` and leave debug keyed as today, or take the cheaper answer
      `crates/nvs-cli/src/cache.rs:164`'s *Known gaps* already names, which keys on the running
      executable rather than on a compile-time walk. Write what it spends either way.
- [ ] **`env_hash` reaches a configured `opcache.file_cache_dir` too** — `crates/nvs-cli/src/cache.rs:164`,
      same rule. `default_dir` compensates for the stale stamp by keying its directory on the running
      executable and a written directory does not, so the compensation goes when the stamp above lands.

## Backlog

- `crates/nvs-server/src/route.rs` gap 1, where a forged CSRF token is refused — stage 5, decided.
- `crates/nvs-server/src/metrics.rs` gap 1, the `otlp` pusher behind `[metrics] endpoint` — stage 10.
- `crates/nvs-server/src/schedule.rs` gap 1, a fire's context carries the deployment's configuration — stage 9.
- `crates/nvs-server/src/trace.rs` gap 1, the sampled-request event gate without `DebugFlags::TRACE` — stage 11.
- `crates/nvs-db/src/catalog.rs` gaps 1–3, `ddl.rs` gaps 1–2, `schema.rs` gap 1 — stage 5, decided.
- Spec § 13's `Core\Test` cell spells `request`'s bag nowhere — `docs/agent/carried-gaps.md` holds it.
