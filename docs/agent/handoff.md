# Handoff

## State

**Goal `cache-shared-dial`, stage 4 landed: `rediss://` is the third transport, and gap 1 is
closed.** `[cache.shared] url` reads three schemes — `crates/nvs-stdlib/src/cache.rs:477`'s `READS`
is the one string every refusal quotes — and `Target::Tls { address, name }`
(`crates/nvs-stdlib/src/cache.rs:2164`) carries the resolved address beside the name the peer's
certificate is checked against. `Transport::dial` (`crates/nvs-stdlib/src/cache/redis.rs:526`)
connects the same `NvsTcp`, sets the socket's own deadline and hands it to `NvsTls::over`: no
relaxation is asked for, per `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`, and it is
`rule:security/one-tls-client`'s client and not a second.

Two spellings that give one value two homes are refused with a sentence naming the key instead: a
path (`[cache.shared] database`) and, new here, userinfo (`[cache.shared] password`). The userinfo
refusal builds its own text rather than going through `refuse`, which quotes the URL — a refusal
about a credential in a URL would otherwise be the first thing to carry it.

`crate::tests::outbound_client` (`crates/nvs-stdlib/src/lib.rs:481`) is now the one place the
process's outbound TLS client is built; `http::transport::tests::trusted` calls it. The playbook
bullet above is why.

## Next group

**Stage 5: the handshake is part of every connection, and a real store agrees** — one file set:
`crates/nvs-stdlib/src/cache/redis.rs`, `examples/cache-shared-tls.nvs`,
`examples/cache-shared-tls.toml`.

- [ ] **`AUTH` then `SELECT` go out before the first command, and again after a dropped socket** —
      `crates/nvs-stdlib/src/cache/redis.rs:209`'s `handshake`, reached from
      `crates/nvs-stdlib/src/cache/redis.rs:180`'s `ensure`, asserted by a fake store in the test
      module at `crates/nvs-stdlib/src/cache/redis.rs:840` as
      `auth_and_select_precede_the_first_command_in_that_order` and
      `a_reconnect_after_a_dropped_socket_sends_both_again`. The reconnect half is the claim the
      keystone exists for: a handshake applied at boot passes the first and fails the second.
      `rule:config/cache-shared-is-the-grant-over-the-configured-store` is the door it comes
      through.
- [ ] **`examples/cache-shared-tls.nvs` and its own `examples/cache-shared-tls.toml`** — the
      acceptance fixture the driver reports missing, written under
      `examples/cache-shared-socket.toml:1`'s reading (its own tree, because `[cache.shared]` is
      unscoped). It dials `rediss://127.0.0.1:16380` — `tests/db/compose.yaml:293`'s `redis`
      already serves TLS there under the `certs` chain, with no compose edit — at a non-zero
      `database`, and prints exactly `put and get agree over TLS` then `the entry is at the
      configured index and not at zero`.

## Backlog

- The goal prose's stage-5 anchor `crates/nvs-stdlib/src/cache.rs:2770` names a `getSecret` doc
  comment, not a scripted store; the wire claims belong in `crates/nvs-stdlib/src/cache/redis.rs`'s
  test module (`docs/agent/loop-goal.md` § *Stage 5*).
- `[cache.shared] database` has no boot-time validation of its own; the tree's `u32` is the whole
  refusal and a store's own `-ERR` is the rest (`docs/agent/loop-goal.md` § *Stage 3*).
- `website/src/data/rules.json` and `website/src/content/docs/.../stores-and-caches.md` still carry
  the pre-TLS divergence sentence; `python tools/rules.py --render` does not write the mirror.
