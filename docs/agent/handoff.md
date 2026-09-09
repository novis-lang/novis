# Handoff

## State

**Goal 24 — `Core\Net`, `Core\Os` and `Core\Signal`. The capability layer is complete; no `Core`
member exists yet.** `net.listen` and `net.local` are both on the roster
(`crates/nvs-config/src/capability.rs:68`), and `Scope::Endpoint(SocketAddr)` at
`crates/nvs-config/src/capability.rs:156` is what a bind is asked at: an entry is an `address:port`
literal, **both sides are parsed**, and an entry that is not an endpoint matches nothing.
`net.listen` carries no address policy, per
`rule:security/net-listen-is-a-separate-grant-from-net-connect`.

`crates/nvs-stdlib/src/net.rs` does not exist and `registry::CLASSES` declares no `Core\Net`, so
`examples/net-echo.nvs` — the driver's earliest failing check — cannot be written until the two
members below land. [0162](../decisions/0162.md) is the goal's one ADR number, spent; its § 7 (the
five entry points) and § 12 (a socket closes with its request) are now in `[context] adrs`, in both
`loop-goal.toml` and the goal's own file.

The four rules 0162 creates are still `designed` with `guardedBy: []`; each names its guard in the
record's *Verification*, and the two `-p nvs-config` tests that landed are the config half of the
first one.

## Next group

**Stage 3: `Core\Net` over TCP, and the fixture that proves it** — one file set:
`crates/nvs-stdlib/src/net.rs` (new), `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-host/src/net.rs`, `crates/nvs-runtime/src/capability.rs`.

- [ ] **`Core\Net::connect` over `NvsTcp`.** 0162 § 7 is the five entry points and § 12 the
      lifetime; `crates/nvs-host/src/net.rs:180` is `NvsTcp` and
      `crates/nvs-stdlib/src/registry.rs:1342` is `CLASSES`, where a class declares its rows. The
      host is pinned through `nvs_runtime::capability::pin_host`
      (`crates/nvs-runtime/src/capability.rs:150`), which asks `net.connect` and then the
      denied-range table — `rule:core-classes/net-one-api-three-transports`. The member's rows come
      out of `crates/nvs-stdlib/tests/migration-members-outstanding.txt` and
      `crates/nvs-stdlib/tests/spec-members-outstanding.txt` in the same slice.
- [ ] **`Core\Net::listen` over `NvsListener`,** `crates/nvs-host/src/net.rs:348`, asking
      `Cap::NetListen` at `Scope::Endpoint` (`crates/nvs-config/src/capability.rs:156`) and never
      `net.connect` — `rule:security/net-listen-is-a-separate-grant-from-net-connect`.
      `a_tcp_listener_asks_net_listen_and_not_net_connect` is the name the `-p nvs-stdlib`
      acceptance check gives it.
- [ ] **`examples/net-echo.nvs`,** which both members above are for. The acceptance `files` list is
      the only place naming it (`docs/agent/loop-goal.toml:9`), so nothing freezes its shape and the
      slice that writes it chooses one — an echo over loopback, granted `listen` and `connect`.

## Backlog

- UDP and the Unix transport — 0162 §§ 10-11; `crates/nvs-host/src/net.rs` has no `NvsUdp` yet.
- The four rules 0162 creates stay `designed` with `guardedBy: []` until their guards land —
  `docs/decisions/0162.md` § *Verification* names each one.
- `Core\Os`, `Core\Signal` and `Core\Budget` — later stages, `rule:core-api/tier-roster`.
- `rule:config/net-local-is-named-and-not-on-the-roster`'s slug contradicts the roster it now sits
  on; the fragment's body is current, so only the id and its citations are stale —
  `docs/rules/config.json`.
