# Handoff

## State

**Goal 24 — `Core\Net`, `Core\Os` and `Core\Signal`. `Core\Net`'s TCP half is on disk and green.**
`crates/nvs-stdlib/src/net.rs` registers three classes — `Core\Net` (`connect`, `listen`),
`Core\Net\Stream` (`read`, `write`, `close`) and `Core\Net\Listener` (`accept`, `port`, `close`) —
over `nvs_host::NvsTcp` and `NvsListener`. `examples/net-echo.nvs` runs, and the driver's earliest
failing check is closed. Two of `rule:core-classes/net-one-api-three-transports`'s five entry points
are built; the datagram and the two Unix-domain ones are not.

The socket lives in the request's own table: `nvs_runtime::Ctx::hold_open_socket` /
`open_socket_mut` / `take_open_socket` over a new `HeldSocket` trait
(`crates/nvs-runtime/src/ctx/held.rs:37`), which is `HeldConnection`'s shape for
`HeldConnection`'s reason — `nvs-host` depends on `nvs-runtime`, so the field is a `dyn Trait` and
`net.rs`'s `Connected`/`Bound` newtypes are what implement it. A handle's one slot holds the key.

Both new rules now carry guards and both are still `designed`, honestly: each states a surface
whose UDP and Unix halves are unwritten. `§16 Core\Net` is struck from
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`.

Two design calls this goal's standing form left to the module doc, both recorded there:
**every waiting member takes a required `Core\Time\Duration`** — there is no configuration block to
default from and no spelling for an unbounded wait — and **`read` answers `tainted bytes` capped by
`READ_CEILING`**, since a socket reports what arrived rather than what was asked for.

## Next group

**Stage 2 (cont.): UDP and the Unix-domain pair, `Core\Net`'s remaining three entry points** — one
file set: `crates/nvs-stdlib/src/net.rs`, `crates/nvs-host/src/net.rs`,
`crates/nvs-stdlib/src/registry.rs`.

- [ ] **A datagram socket, over the reactor.** `nvs-host` has no UDP source yet — `NvsStream` is
      generic over `mio::event::Source` (`crates/nvs-host/src/net.rs:149`) and
      `mio::net::UdpSocket` fits it, so the slice is an alias plus `send_to`/`recv_from` beside
      `poll_read` (`crates/nvs-host/src/net.rs:755`). `rule:core-classes/net-udp-carries-no-reliability-layer`
      and 0162 § 11 are the surface; the two `-p nvs-host` names the acceptance check wants are
      `a_udp_socket_registers_with_the_reactor_and_parks_the_coroutine` and
      `a_udp_read_with_no_datagram_waiting_suspends_rather_than_spinning`.
- [ ] **`Core\Net::bindDatagram` and its class, asking two grants.**
      `rule:security/net-listen-is-a-separate-grant-from-net-connect` is the one that matters here:
      `net.listen` once for the local port, and `net.connect` re-asked through
      `nvs_runtime::capability::pin_host` (`crates/nvs-runtime/src/capability.rs:153`) at **every**
      send. Register beside `crate::net::LISTENER` (`crates/nvs-stdlib/src/registry.rs:1699`).
      Acceptance name: `a_udp_socket_sends_and_receives_over_the_runtimes_own_reactor`.
- [ ] **`Core\Net::connectPath` and `listenPath` over `NvsUnix`.**
      `crates/nvs-host/src/net.rs:486` is `NvsUnix` and `:513` its `connect`; there is no
      Unix-domain listener there yet. Both ends ask `Cap::NetLocal` at
      `Scope::Path` — `rule:config/net-local-is-named-and-not-on-the-roster` and 0162 § 10, which is
      where "it governs binding as well" is settled. Acceptance names:
      `a_program_supplied_unix_path_is_refused_as_a_target_at_every_door` and
      `no_door_dispatches_on_a_url_scheme`; the second is the whole five-member surface at once and
      wants the other four to exist first. `#[cfg(unix)]` — `NvsUnix` is not built on Windows, so
      the members need a refusal arm naming the platform there.

## Backlog

- Stage 3 `Core\Os` and stage 4 `Core\Signal` — `docs/agent/loop-goal.toml`'s stages 3 and 4 name
  their tests; `Core\Budget` and `Core\Os` still hold rows in
  `crates/nvs-stdlib/tests/migration-members-outstanding.txt`.
- `rule:core-classes/net-one-api-three-transports` and
  `rule:security/net-listen-is-a-separate-grant-from-net-connect` flip to `shipped` when the
  remaining three entry points land — `docs/rules/*.json`.
- `Core\Net\Stream` answers no peer address; `stream_socket_get_name`'s remote half has no member —
  `docs/spec/02-php-migration.md:775`.
- A read on a served request parks a core on a peer's schedule; nothing caps the number of sockets
  a request may open — `docs/agent/carried-gaps.md`, goal 36.
