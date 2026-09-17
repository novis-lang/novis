# Handoff

## State

**Goal 65 — a password, a database index and TLS reach the shared store — has just started; nothing of it has landed yet.** Goal `tds-bytes`'s whole list is this goal's Stage 1 floor.

What is settled before the first session: none of the three needs a mechanism built. The secret
plumbing is `crates/nvs-config/src/secret.rs:161`'s `SECRETS`, one `SecretPair` row per
`value`/`value_file` pair; the TLS client is `nvs_host::tls::NvsTls`, the process's only one
(`rule:security/one-tls-client`), already dialled over a plain socket at
`crates/nvs-stdlib/src/http/transport.rs:1633`; and `Transport`
(`crates/nvs-stdlib/src/cache/redis.rs:433`) is already the enum that says which socket a store is
reached over. `tests/db/compose.yaml:293`'s `redis` serves TLS on `6380` today, published at
`127.0.0.1:16380`, so stage 5 needs no compose edit.

A session must not re-decide three things (the goal's *Standing decisions*): the credential is the
`password`/`password_file` pair and never URL userinfo, the index is the `database` key and never a URL
path, and no trust relaxation reaches this store.

## Next group

**Stage 2: the dial** — one file set: `crates/nvs-stdlib/src/cache.rs`,
`crates/nvs-stdlib/src/cache/redis.rs`.

- [ ] **The dial is one value, not three** — `crates/nvs-stdlib/src/cache.rs:2115`'s `Target` says
      which socket and nothing else, and `crates/nvs-stdlib/src/cache/redis.rs:448`'s
      `Transport::dial` takes it. What the dial takes becomes transport, credential and index
      together, settled once where the URL is read.
- [ ] **`connect` applies the whole of it** — `crates/nvs-stdlib/src/cache/redis.rs:172` dials on the
      first command and again after a dropped socket, so it is the one place all three are applied.
      A session that authenticates anywhere else has built the reconnect bug this stage exists to
      stop.
- [ ] **A store configured with neither dials as it did** — the same file, the arm where `password`
      and `database` are both absent, which is every deployment on the chain today and must not gain
      a round trip.

## Backlog

- **Stage 3, the two keys** — `crates/nvs-config/src/secret.rs:161`'s `SECRETS` gains the pair and
  `crates/nvs-config/src/tree.rs:1273`'s `CacheShared` gains `database`, both `System`-class in
  `crate::directive`'s `cache.shared` row. File set: `crates/nvs-config/src/`.
- **Stage 4, the transport arm** — `crates/nvs-stdlib/src/cache/redis.rs:433`'s `Transport` gains a
  TLS arm over `NvsTcp`, agreeing with `crates/nvs-stdlib/src/http/transport.rs:1633` rather than
  inventing a second door. Same file set as stage 2.
- **Stage 5, every connection and a real store** — the scripted store at
  `crates/nvs-stdlib/src/cache.rs:2770` for the order and the reconnect, then
  `examples/cache-shared-tls.nvs` against the container.
- **Stage 0's six sentences** — each rewritten in the slice that makes it wrong, not after it; the
  goal's *Stage 0* lists them with their files, and `crates/nvs-stdlib/src/cache.rs:155` gap 1 is the
  last of them.
- When this goal's last check goes green the driver takes goal `gap-zero`.
