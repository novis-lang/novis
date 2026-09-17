# Handoff

## State

**Goal `cache-shared-dial`, stage 2 landed: the dial is one value and every connection applies the whole
of it.** `crates/nvs-stdlib/src/cache.rs:2161`'s `Dial` carries the target, the credential and the
database index; `redis::Connection` holds one where it held a `Target`; `open_shared` compares it to
decide reuse-or-replace; and `Connection::ensure` sends `AUTH` then `SELECT` on every socket it opens,
including the silent reconnect `Connection::command` makes behind a dropped one.

Nothing sets either value yet — `Dial::to` is the store a URL named and nothing beside it, which is what
both readers of a URL (`open_configured` and `Lease::open`) build today. Stage 3 is the two sources.

Stage 0's catch-up sentences are all still on disk and still true of the shipped configuration surface;
each is rewritten in the slice that makes it wrong, per the goal file. The floor is goal `tds-bytes`'s
list, untouched.

The pack's `[context] modules` did not print `crates/nvs-stdlib/src/ratelimit.rs` or
`crates/nvs-stdlib/src/session.rs`, both of which build a `redis::Connection` in their test modules; this
session's own commits touch them, so the driver's sweep closes it.

## Next group

**Stage 3: two keys, as registry rows** — one file set: `crates/nvs-config/src/secret.rs`,
`crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/default.toml`,
`crates/nvs-stdlib/src/cache.rs`.

- [ ] **`[cache.shared] password` is a `SecretPair` row** — `crates/nvs-config/src/secret.rs:161`'s
      `SECRETS`, gaining its `password_file` sibling and every property
      `rule:config/a-secret-is-a-file-whose-content-is-the-value` already states for the two pairs above
      it: exactly one of the pair, the file's whole content, never in the merged table, `<secret>` in
      `nvs config dump`.
- [ ] **`[cache.shared] database` is an ordinary key** — `crates/nvs-config/src/tree.rs:1273`'s
      `CacheShared`, beside `url` and `timeout`, a non-negative integer, with an index the store does not
      have reported as the store's own refusal. Its block doc and
      `crates/nvs-config/src/default.toml`'s commented `[cache.shared]` keys name two keys and must name
      all four. `crates/nvs-config/src/directive.rs:189`'s `cache.shared` row is already `System` and
      covers both, per the goal's stage 3.
- [ ] **The door reads both into the dial** — `crates/nvs-stdlib/src/cache.rs:2228`'s `open_configured`
      builds the `Dial` where it reads the URL, so both belong there beside `endpoint`. `Lease::open` in
      the same file is the second reader and has no `Ctx`: it takes the URL and the timeout as text
      (`bound_of`), so the credential reaches it the same way or the fleet lease dials uncredentialed.

## Backlog

- Stage 4 — `rediss://` as `Target`'s and `Transport`'s third arm over `nvs_host::tls::NvsTls`
  (goal file § *Stage 4*).
- Stage 5 — `AUTH`/`SELECT` over a real store and an `examples/cache-shared-socket.nvs` sibling
  (goal file § *Stage 5*).
- Stage 0's catch-up list — `crates/nvs-stdlib/src/cache.rs:155` gap 1, `READS`, the URL refusal's text,
  `CacheShared`'s doc, `default.toml`, the `docs/agent/carried-gaps.md` § *Owned* row.
