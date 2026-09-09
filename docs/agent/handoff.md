# Handoff

## State

**Goal 24 — `Core\Net`, `Core\Os` and `Core\Signal`. Stage 2's design half has landed; no member
exists yet.** [0162](../decisions/0162.md) is the goal's one ADR number, spent: it decides the
socket surface as five entry points over three transports, and decides that **the grant follows the
direction rather than the transport** — reaching out is `net.connect` with
`rule:security/net-address-policy`'s table, binding is `net.listen` with none, a socket path at
either end is `net.local`. A UDP socket needs two grants, because it does two things.

`net.local` is on the roster and exercised at the config layer
(`crates/nvs-config/src/capability.rs:73`, `crates/nvs-config/src/tree.rs:298`). **`net.listen` is
not on it yet** — it has no scope variant, and the stage that builds a listener is the one that
should choose it. `Scope` has `Unscoped | Path | Host | Name` and none of them is an `address:port`.

The four rules 0162 creates are `designed` and `guardedBy: []`; each names its guard in the record's
*Verification*. `Core\Socket` (`crates/nvs-stdlib/src/socket.rs`) is still the WebSocket upgrade and
is not what this goal extends.

## Next group

**Stage 3: TCP, and the grant a bind asks** — one file set: `crates/nvs-config/src/capability.rs`,
`crates/nvs-config/src/tree.rs`, `crates/nvs-host/src/net.rs`, and the new
`crates/nvs-stdlib/src/net.rs`.

- [ ] **`net.listen` joins the roster, and picks its scope.**
      `rule:security/net-listen-is-a-separate-grant-from-net-connect` says entries are `address:port`
      literals matched exactly and that the grant carries no address policy;
      `crates/nvs-config/src/capability.rs:109` is the `Scope` enum that has no variant for one, and
      `crates/nvs-config/src/capability.rs:255` is `is_path_scoped`, the shape a per-capability
      property takes here. `net_local_and_net_connect_are_separate_grants` at
      `crates/nvs-config/tests/capability.rs:547` is the test to mirror, from the third side.
- [ ] **`Core\Net::connect` over `NvsTcp`.** `crates/nvs-host/src/net.rs:313` is `NvsTcp::connect`
      with its deadline, and `crates/nvs-stdlib/src/json.rs` is the five-edit module small enough to
      read whole. The door asks `Cap::NetConnect` at `Scope::Host` and then the address, in that
      order — `crates/nvs-runtime/src/capability.rs` holds `require` and `pin_host`.
- [ ] **`Core\Net::listen` over `NvsListener`,** whose accepting half is at
      `crates/nvs-host/src/net.rs:327`. `rule:core-classes/net-one-api-three-transports`: accepting
      is a member on the listener, never a sixth entry point.

## Backlog

- Stage 4: UDP needs `NvsUdp` in `crates/nvs-host/src/net.rs` — that crate has no datagram type, and
  it is the one addition to it (`rule:core-classes/net-udp-carries-no-reliability-layer`).
- Stage 5: Unix needs no new transport — `NvsUnix` is at `crates/nvs-host/src/net.rs:478`; the door
  asks `Cap::NetLocal` at `Scope::Path`, both ends (`rule:config/net-local-is-named-and-not-on-the-roster`).
- Stage 6: `Core\Os`, `Core\Signal` and `Core\Budget` — `rule:core-api/tier-roster` rows with no
  design question left; [0148](../decisions/0148.md) settled the one that existed.
- The migration table's `Core\Net` cells name no `Core\X::member` spelling, so they owe no key in
  `crates/nvs-stdlib/tests/migration-members-outstanding.txt` until one is written there.
- `docs/agent/loop-goal.toml`'s `[context]` printed everything this session needed; no field was
  missing.
