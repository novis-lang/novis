# Handoff

## State

**Goal `websocket-client` — a program holds a WebSocket to another server, opened like an outbound
call and closed with the task that opened it.** Stage 1 (goal `outbound-proxy`'s list) is the floor
and passes; stage 2's record, [ADR 0183](../decisions/0183.md), is accepted; stage 3's scripted peer
is on disk and green.

**Stage 4's code is landed, and four of the six tests its `cargo-named` check pins are green** —
the two handshake cases, the unoffered-subprotocol refusal and the redirect. `openSocket` branches on
`ctx.faked_http().is_armed()`: the armed arm keeps the table's `LogicError`, and the unarmed arm
assembles a `transport::Call` and calls `transport::upgrade`, which dials through
`transport::dialled` — one connect implementation, shared with `one` — and runs `tungstenite`'s
`client_with_config` over the connection it left. Nothing is pooled, the live conversation is filed
in the request's table under the class's `held` slot, and `close` gives the connection back.

**The subprotocol judgement is now `transport::settled`**, which both arms pass through: the live one
inside `upgrade`, the scripted one at the call site in `socket.rs`. `socket.rs`'s own `offered` is
gone. `tungstenite` asks the same question of a live `101` and gets there first, refusing without
naming the name — so the wording is asserted of `settled` directly, and the live case asserts only
that the `101` was refused.

**What is not real yet is the conversation.** `receive`, `send` and `sendBytes` still read and write
the *scripted* table, so a socket opened against a real host completes its handshake and then reads
as a peer that said nothing — stage 5's work, with `maxMessage`, `idle`, `maxDuration`, the send wait
and `ping` (judged, never applied) and `close`'s `$code`/`$reason` (accepted, never sent).

## Next group

**Stage 4: the last two loopback tests, which is what the stage's first check still owes** — one file
set: `crates/nvs-stdlib/src/http/transport.rs` (its `mod tests`, from line 2772).

- [ ] **A TLS socket origin, then `wss_socket_completes_over_the_tls_client_under_the_calls_policy`** —
      `rule:security/one-tls-client`, [ADR 0183](../decisions/0183.md) § 4.
      `crates/nvs-stdlib/src/http/transport.rs:4810` (`tls_origin`) and
      `crates/nvs-stdlib/src/http/transport.rs:4782` (`trusted`) are the helpers, but `answer_tls`
      writes a **static** reply and so cannot open a socket: the `101`'s accept has to be derived per
      request, which is what `crates/nvs-stdlib/src/http/transport.rs:2894` (`handshake`) already
      does over a plain stream. The cheapest shape is a `tls_origin` that takes a reply *closure*, or
      a sibling that runs `handshake`'s body over a `rustls::Stream`. `wss` reaches TLS through
      `crates/nvs-stdlib/src/http/transport.rs:2476` (`parts`), and the policy the case sets is
      `CallPolicy`, as `crates/nvs-stdlib/src/http/transport.rs:4887` sets it.
- [ ] **`a_socket_is_opened_through_the_proxy_tunnel_when_one_is_configured`** —
      `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`. The loopback
      proxy at `crates/nvs-stdlib/src/http/transport.rs:2951` (`proxy`) pumps octets both ways
      untouched, so the `ws` handshake crosses it as a TLS flight does; `through` at
      `crates/nvs-stdlib/src/http/transport.rs:2930` is the `[http.client.proxy]` block, and
      `socket_origin` at `crates/nvs-stdlib/src/http/transport.rs:2853` is the far end. Assert the
      `CONNECT` head names the pinned address and that the socket opened behind it.

## Backlog

- Stage 5 is the whole conversation: `receive`/`send`/`sendBytes` over `Open` rather than the
  scripted table, and the bounds `openSocket` already judges — `docs/agent/loop-goal.md` § stages.
- `permessage-deflate`, RFC 8441, reconnection and subprotocol libraries stay out — the goal's
  § *Standing decisions* says so.
