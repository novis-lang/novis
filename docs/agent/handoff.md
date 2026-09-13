# Handoff

## State

**Goal `websocket-client` — a program holds a WebSocket to another server, opened like an outbound call
and closed with the task that opened it.** Stage 1 (goal `outbound-proxy`'s whole list) is the floor and
passes. Stage 2's record, [ADR 0183](../decisions/0183.md), is on disk and accepted.

**Stage 3 is done and its acceptance check is green.** `Core\Http\Client::openSocket` and
`Core\Http\Socket` are registered, the scheme roster is four, and `Core\Test::answerSocket` /
`::sentSocket` script a peer and read back what the program sent. Five `.nvst` cases cover it, including
the two the goal's stage-3 check names.

**What is on disk is the table path only.** `openSocket` refuses every URL no scripted peer answers,
because there is no `is_armed` branch yet: stage 4 adds the real handshake behind one, and that is the
one place the refusal's wording ("this test answers outbound sockets from a table") becomes wrong. The
bag's bounds are *judged* — a non-positive `idle`, `maxDuration`, `sendTimeout` or `ping` throws — and
none of them is *applied*, which is stage 5's; `close`'s `$code` and `$reason` are accepted and not
sent, for the same reason.

**Spellings this stage fixed, which no later stage re-opens:** the socket bag is `SOCKET_OPTIONS`
(`crates/nvs-stdlib/src/http.rs`), built by the new `connection_options!` macro — the shared connection
keys — with `request_options!` writing the exchange half into its middle; the card's entries are
`SOCKET_PARAMS` over `connection_params!`, the same split one axis over. The socket bag's ABI slots are
`SOCKET_DEADLINE`…`SOCKET_PING` and are **not** `DEADLINE`'s numbers.

## Next group

**Stage 4: the handshake against a real host** — one file set: `crates/nvs-stdlib/src/http/socket.rs`,
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/http.rs`.

- [ ] **Branch `openSocket` on the armed table**, so the scripted path is the exception and the
      connect is the rule — `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`,
      [ADR 0183](../decisions/0183.md) § 4. The body is
      `crates/nvs-stdlib/src/http/socket.rs:251`; the guard to copy is `crates/nvs-stdlib/src/http.rs:3221`
      (`ctx.faked_http().is_armed()`), and the refusal that moves inside the armed arm is the
      `Fault::thrown_as` below the `socket_for` lookup. Its message names a test, so it may only be
      reached from one.
- [ ] **Open the connection through the transport and run `tungstenite`'s client half over it** —
      the pin, every approved address, `connectTimeout` and the proxy tunnel, with no adapter and no
      new dependency. `crates/nvs-stdlib/src/http/transport.rs:1237` is `send`, whose connect and TLS
      hand-off this reuses; `crates/nvs-stdlib/src/http.rs:3516` is `exchanged`, the shape that picks
      between the table and the wire today.
- [ ] **The `101` and what refuses it** — a `3xx` is a `RuntimeError` naming the `Location` and never
      a hop taken, any other status is a `RuntimeError` naming it, and a chosen subprotocol that was
      never offered is refused where the scripted path already refuses it
      (`crates/nvs-stdlib/src/http/socket.rs:251`, the `offered` call). A socket is **never pooled**:
      it consumes its connection and nothing goes back.

## Backlog

- The four bounds are judged but not applied, and `maxMessage` is neither — stage 5, goal
  `websocket-client`'s own § *Stage 5*.
- `[http.client.socket] max_message` / `send_timeout` are not on disk — stage 5, and
  `crates/nvs-config/tests/tree.rs` is where the block is asserted.
- `close`'s `$code` and `$reason` are accepted and dropped; nothing sends a close frame — stage 5.
- `Core\Socket\Message`'s module doc still says it is the shape both of `receive`'s *sources* answer
  in — stage 5 rewrites it whole (`crates/nvs-stdlib/src/socket.rs:446`).
- Stage 6 flips ADR 0183's four rules to `shipped` with `guardedBy` filled from this goal's cases.
- The `nvs/rest` package is still unscheduled — [carried-gaps.md](carried-gaps.md).
