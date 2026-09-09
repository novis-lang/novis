# Handoff

## State

**Goal 24 — `Core\Net`, `Core\Os` and `Core\Signal`. The datagram transport now exists in
`nvs-host`; `Core\Net` still registers only the TCP two of its five entry points.**
`crates/nvs-host/src/net.rs:493` is `NvsUdp`, a third alias over `NvsStream` — `bind`, `from_std`,
`local_addr`, `send_to` and `recv_from`, parking on the same four functions and carrying no `Read`
and no `Write`, because a datagram socket has no stream to read. It is re-exported as
`nvs_host::NvsUdp`. Both names the stage-2 `-p nvs-host` check asks for are green.

`crates/nvs-stdlib/src/net.rs` is untouched: `Core\Net` is `connect` and `listen`, and its module doc
still says UDP and the two Unix-domain doors are not on disk. Nothing here is blocked — the next
group is ordinary building, and the one design question it had is answered below rather than left
for the session that hits it.

Only one slice was taken. The stdlib datagram surface is **all-or-nothing** —
`conformance_coverage.rs`'s three-cases-per-member floor fails the build for a member that lands
without its cases — so a half-written class leaves a red tree, and it was not worth starting at the
context this session had left.

## Next group

**Stage 2 (cont.): the datagram surface in `Core\Net`, over the `NvsUdp` that landed** — one file
set: `crates/nvs-stdlib/src/net.rs`, `crates/nvs-stdlib/src/registry.rs`,
`tests/conformance/core/`.

- [ ] **`Core\Net::bindDatagram` and `Core\Net\Datagram`, asking the two grants.**
      `bindDatagram(string $address, uint $port): Core\Net\Datagram` is
      `crates/nvs-stdlib/src/net.rs:706`'s `listen` body with `NvsUdp::bind` — an address literal,
      no DNS, `Cap::NetListen` at `Scope::Endpoint`. The class is `port`, `close`,
      `send(string $host, uint $port, bytes $payload, Duration $within): uint` and
      `receive(uint $max, Duration $within)`. **`send` asks the second grant**:
      `nvs_runtime::capability::pin_host` exactly as `connect` does at
      `crates/nvs-stdlib/src/net.rs:673`, because `rule:security/net-address-policy` says a datagram
      sent to a program-supplied address is asked at the send what a TCP connect is asked at the
      connect. Rows and cards at `crates/nvs-stdlib/src/net.rs:109`, the `address()` arm at
      `crates/nvs-stdlib/src/net.rs:450`, the roster at `crates/nvs-stdlib/src/registry.rs:1705`
      and its instance rows at `crates/nvs-stdlib/src/registry.rs:2035`.
- [ ] **`receive` answers a `Core\Net\Datagram\Message`, and that is the decision to record in the
      module doc.** Three slot-reading members — `payload(): tainted bytes`, `host(): string`,
      `port(): uint` — no capability, no waiting. The row `bindDatagram` replaces is
      `stream_socket_recvfrom`, which delivers the sender through an out-param; Novis has neither
      out-params nor tuples, so an object is the narrowest thing that answers it, and a `receive`
      answering bytes alone would be a datagram socket that cannot reply. `payload` is capped by
      `READ_CEILING` for `crates/nvs-stdlib/src/net.rs:100`'s reason.
- [ ] **The cases, which are where this slice's cost is.** Three `.nvst` files reach every new
      member's floor of three, on
      `tests/conformance/core/net-a-tcp-echo-round-trips-over-the-reactor.nvst:1`'s pattern — two
      sockets in one program, send then receive, so nothing parks with nobody to wake it. Loopback
      needs `net.internal` beside `net.connect`, so each is a multi-file case. Plus the
      `-p nvs-stdlib` name the acceptance check wants,
      `a_udp_socket_sends_and_receives_over_the_runtimes_own_reactor`, beside the two that already
      assert which door refuses at `crates/nvs-stdlib/src/net.rs:885`.

## Backlog

- `Core\Net::connectPath` and `listenPath` over `NvsUnix`, asking `net.local` —
  `rule:config/net-local-is-named-and-not-on-the-roster`, stage 2's third item.
- `rule:core-classes/net-one-api-three-transports` and
  `rule:core-classes/net-udp-carries-no-reliability-layer` stay `designed` until all five doors land.
- `§16 Core\Net` is struck from `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`
  already; nothing to do there.
- Stages 3-6 (`Core\Os`, `Core\Signal`, `Core\Budget`) are untouched — `docs/agent/loop-goal.toml`.
