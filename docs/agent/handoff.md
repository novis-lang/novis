# Handoff

## State

**Goal `cache-shared-dial`, stage 3 landed: both keys exist, and every connection in the process
builds the whole dial from them.** `[cache.shared] password` is a `SecretPair` row
(`crates/nvs-config/src/secret.rs:270`) with its `password_file` sibling, so the credential arrives
as a mounted file, stays out of the merged table and renders `<secret>`; `[cache.shared] database`
is an ordinary `u32` beside `url` and `timeout` (`crates/nvs-config/src/tree.rs:1299`). The door
(`crates/nvs-stdlib/src/cache.rs:2249`) and the fleet lease (`crates/nvs-stdlib/src/cache.rs:2411`,
called from `crates/nvs-cli/src/serve.rs:1424`) both settle a whole `Dial`, and
`Dial::configured` is its one constructor — the shorthand the wire's own cases used is gone, so a
key added later cannot be dropped by a caller that took the short way.

A credential is read **verbatim** — `cache::configured` trims and `cache::credential` deliberately
does not, per `rule:config/a-secret-is-a-file-whose-content-is-the-value`.

What is left of gap 1 is TLS alone: `rediss://` is still refused, and the module doc, `CacheShared`'s
doc, `default.toml`, the rule fragment and the `carried-gaps.md` row have each been cut back to that
one half. Stage 4 makes them wrong again, and strikes the row.

The pack's `[context] modules` did not print `crates/nvs-cli/src/serve.rs`, which holds the fleet
lease's one call site; this session's commits touch it, so the driver's sweep closes it.

## Next group

**Stage 4: `rediss://` is the third transport arm** — one file set:
`crates/nvs-stdlib/src/cache.rs`, `crates/nvs-stdlib/src/cache/redis.rs`,
`crates/nvs-host/src/tls.rs`.

- [ ] **`Target` gains a TLS arm and the URL reader accepts the scheme** —
      `crates/nvs-stdlib/src/cache.rs:2130`'s enum, `crates/nvs-stdlib/src/cache.rs:2025`'s
      `endpoint`, which today refuses `rediss://` in the `else` of its `redis://` strip, and
      `crates/nvs-stdlib/src/cache.rs:472`'s `READS`, the one string every refusal quotes, on both
      the Unix and the non-Unix build. `rule:security/one-tls-client` is what it must not become a
      second of.
- [ ] **`Transport` gains the arm that dials it** — `crates/nvs-stdlib/src/cache/redis.rs:490`:
      `NvsTcp` as it always was, handed to `nvs_host::tls::NvsTls`
      (`crates/nvs-host/src/tls.rs:177`) for a handshake that gives back a plaintext stream, which
      is `crates/nvs-stdlib/src/http/transport.rs:507`'s door. The `Read`, `Write` and
      `set_deadline` matches gain their arm and nothing else about the connection moves. No trust
      relaxation reaches this store — the goal's § *Standing decisions* is why, and
      `rule:security/tls-trust-is-relaxed-only-under-a-host-grant` is the rule.
- [ ] **The sentences TLS makes wrong, rewritten in this slice** — gap 1 at
      `crates/nvs-stdlib/src/cache.rs:155`, `CacheShared`'s doc at
      `crates/nvs-config/src/tree.rs:1265`, `crates/nvs-config/src/default.toml`'s `[cache.shared]`
      block, `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host` (fragment plus
      `docs/rules/config.json`'s `divergesFromPhp`, then `python tools/rules.py --render`), and the
      `cache-shared-dial` row in `docs/agent/carried-gaps.md`, struck when the gap closes.

## Backlog

- Stage 5: the scripted store asserts `AUTH` then `SELECT` on the first connect **and on the
  reconnect** — `crates/nvs-stdlib/src/cache.rs`, the scripted-store test module
  (`docs/agent/loop-goal.md` § *Stage 5*).
- Stage 5: `examples/cache-shared-tls.nvs`, the acceptance fixture the driver reports missing, beside
  `examples/cache-shared-socket.nvs` and dialling `rediss://127.0.0.1:16380` at a non-zero index.
- `[cache.shared] database` has no boot-time validation of its own; the tree's `u32` is the whole
  refusal and a store's own `-ERR` is the rest (`docs/agent/loop-goal.md` § *Stage 3*).
