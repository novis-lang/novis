---
milestone: post-parity
---
# Loop goal 65 — a password, a database index and TLS reach the shared store

`[cache.shared]` names a store the way a deployment actually runs one: behind a credential, at a chosen
database index, and over TLS. A `Core\Cache::shared()` entry can therefore live in a managed store on a
network the process does not own, which is the only kind most deployments have. Afterwards
`crates/nvs-stdlib/src/cache.rs` owns no gap.

## Why here

Directly after goal `tds-bytes` and in front of goal `gap-zero`, because that goal's gate is that no
register item names a goal and `crates/nvs-stdlib/src/cache.rs:155` gap 1 names this one.

It is here rather than earlier because each of the three is a row or an arm rather than a mechanism, and
the mechanisms landed on the chain behind it. The secret plumbing the credential needs is
`crates/nvs-config/src/secret.rs:161`'s `SECRETS`, one row per `value`/`value_file` pair with the sites
and the setter beside it, so `[cache.shared] password` is a third row where `[db.*]` and `[mail.*]`
already are. The TLS client a `rediss://` store needs is `nvs_host::tls::NvsTls`, the process's only one
(`rule:security/one-tls-client`), and `crates/nvs-stdlib/src/http/transport.rs:1633` is a caller of it
over a plain socket — the same shape, already written. And the transport choice is already an enum:
`Transport` (`crates/nvs-stdlib/src/cache/redis.rs:433`) is a `Tcp`/`Unix` pair *because* which socket a
store is reached over is read out of `nvs.toml`, so a third arm is what that enum exists for.

What makes the three one goal rather than three is where they are applied. `connect`
(`crates/nvs-stdlib/src/cache/redis.rs:172`) dials on the first command and silently again after a
dropped socket, so the transport, the credential and the index are one thing — the *dial* — or they are
a reconnect that comes back unauthenticated and pointed at database 0.

## Stage 0 — the catch-up

The sentences on disk that go wrong the day this goal is green, each with the file that holds them:

- `crates/nvs-stdlib/src/cache.rs:155` — gap 1 itself.
- `crates/nvs-stdlib/src/cache.rs:2007-2024` — the refusal's own doc and the message it throws, which
  say a `rediss://` is a scheme this client does not speak and that a database index has nowhere to go.
  The half that stays is why a URL *path* is still refused, which the standing decisions below give.
- `crates/nvs-stdlib/src/cache.rs:461-465` — `READS`, the one string every refusal quotes as what a URL
  may be, on both the Unix and the non-Unix build.
- `crates/nvs-config/src/tree.rs:1265-1270` — `CacheShared`'s doc: *the URL and the wait, and nothing
  beside them*, and the sentence deferring all three refusals to `nvs_stdlib::cache`'s module doc.
- `crates/nvs-config/src/default.toml`'s `[cache.shared]` block — the commented keys an operator reads
  first, which name two.
- `docs/agent/carried-gaps.md` § *Owned* — the row naming this goal, struck when the gap closes.

## Stage 1 — the floor

Goal `tds-bytes`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: the dial is one value, applied in one place

`Transport::dial` (`crates/nvs-stdlib/src/cache/redis.rs:448`) takes a `Target` and a timeout, and
`Target` (`crates/nvs-stdlib/src/cache.rs:2115`) is an address or a path — which is the transport and
nothing else. The keystone is that what the dial takes becomes the whole of what the deployment
configured: transport, credential and index together, settled once where the URL is read and carried
into `connect` (`crates/nvs-stdlib/src/cache/redis.rs:172`) as one value.

That is what makes the rest mechanical, and it is the difference between this goal and a session that
authenticates at boot: `connect` is reached again after a dropped socket, and a reconnect that applies a
subset is a connection answering the wrong database under no credentials.

## Stage 3 — two keys, as registry rows

`[cache.shared] password` is a `SecretPair` row in `crates/nvs-config/src/secret.rs:161`'s `SECRETS`,
gaining its `password_file` sibling and every property
`rule:config/a-secret-is-a-file-whose-content-is-the-value` already states for the two pairs above it:
exactly one of the pair, the file's whole content, never in the merged table, `<secret>` in
`nvs config dump`. `[cache.shared] database` is an ordinary key beside `url` and `timeout` on
`CacheShared` (`crates/nvs-config/src/tree.rs:1273`), a non-negative integer, with a store's own refusal
of an index it does not have reported as the store's.

Both are `System`-class in `crate::directive`'s `cache.shared` row, beside the two that are there:
which store a fleet's coherent state lives in is one deployment decision, and a credential is not a
second one.

## Stage 4 — `rediss://` is the third transport arm

`Target` gains a TLS arm and `Transport` (`crates/nvs-stdlib/src/cache/redis.rs:433`) gains the arm that
dials it: `NvsTcp` as it always was, handed to `nvs_host::tls::NvsTls` for a handshake that gives back a
plaintext stream, which is `crates/nvs-stdlib/src/http/transport.rs:1633`'s door and the only outbound
TLS this process has (`rule:security/one-tls-client`). The `Read`, `Write` and `set_deadline` matches
gain their arm and nothing else about the connection moves — the bytes above the socket are the same
bytes, which is the reason that enum is an enum.

## Stage 5 — the handshake is part of every connection, and a real store agrees

The scripted store at `crates/nvs-stdlib/src/cache.rs:2770` is where the wire claim is made: `AUTH` and
`SELECT` go out before the first command, in that order, **and again on the reconnect after a dropped
socket** — which is the claim the keystone exists for and the one a boot-time handshake fails.

Two of the three then meet a real store, and the compose file needs no edit for either.
`tests/db/compose.yaml:293`'s `redis` service already serves TLS on `6380`, published at
`127.0.0.1:16380` under the `certs` service's chain, with `--tls-auth-clients no`; and `SELECT`
reaches the sixteen databases every stock store has. A fixture beside
`examples/cache-shared-socket.nvs` — that one's sibling, written under the same reading — dials
`rediss://127.0.0.1:16380` at a non-zero index and reads its own entry back.

## Standing decisions

- **One spelling per value, so the refusals on disk mostly stay.** The credential is the
  `password`/`password_file` pair and never URL userinfo; the index is the `database` key and never a
  URL path. That is `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s own argument — two
  sources for one value is a refusal, not a convenience — and `rule:errors/ambiguous-input-refused` for
  the index. A session that reads `redis://:pw@host/3` has put a credential in a string that is logged
  and given one value two homes.
- **TLS is a scheme and not a key**, `rediss://` beside `unix:`, because it says which transport rather
  than carrying a value over one. That is the same reason `Target` is a closed enum.
- **No trust relaxation reaches this store.** No `tlsVerify`, no `tlsCa`, no pin on `[cache.shared]`.
  `rule:security/tls-trust-is-relaxed-only-under-a-host-grant` puts every relaxation behind a grant
  proved at a call site with a host, and a configured store is not one; a deployment whose store has a
  private CA adds it to the process's roots, which is the client every other outbound connection
  already checks against.
- **Applied on every connection, never once at boot.** `crates/nvs-stdlib/src/cache/redis.rs:172` is the
  one place, for the reason stage 2 gives.
- **The credential's proof is the scripted store's, and the compose file does not grow a password.**
  `requirepass` is server-wide, so putting one on `tests/db/compose.yaml:293`'s `redis` would demand it
  of every fixture already pointed there — `examples/cache-shared-socket.nvs` and the floors of goals
  `core-part-ii` and `server` — and a second service for one `AUTH` asserts the store twice. The
  scripted store proves the stronger claim anyway: not that a password is accepted, but that it goes
  out first, with the configured value, on every connect including the silent one.
- **The grant does not move.** `rule:config/cache-shared-is-the-grant-over-the-configured-store` is
  unscoped because a deployment has one shared store, and gaining a credential does not make it two.
  This goal adds no capability and no scoping to `cache.shared`.
- **What it spends**, per `rule:programs/memory-priority` and written into the module that takes it: the
  credential is one `String` per process on the config tree, the shape `[db.<name>] password` already
  holds; a TLS store adds one `rustls` connection state per core's one connection, which is the state
  `Core\Http\Client` holds per connection today. Nothing per cache entry, and the shared tier's
  "one socket per core and nothing per entry" stays exactly that.
- **Not this goal**: the local and process tiers, which reach no store; `Core\Session`'s `shared`
  backend, which dials through this same client and gains all three without a line of its own; and a
  second shared store, which the grant rule does not have.
- **ADR slots**: none. The four rules named above state every half, and a rule fragment this goal makes
  wrong is amended in the slice that makes it wrong.
