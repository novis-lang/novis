# Handoff

## State

**Goal `websocket-client` — a program holds a WebSocket to another server, opened like an outbound
call and closed with the task that opened it.** Stages 1–4 are closed: the handshake is live over
plaintext `ws`, over `wss` through the process's one outbound TLS client, and through the operator's
`CONNECT` tunnel, with a bypassed host dialled directly.

**Stage 5's configuration half is landed.** `[http.client.socket]` is a block in
`crates/nvs-config/src/tree.rs:679` carrying `max_message` and `send_timeout`, `Runtime` and `Reload`
in the registry, with `E0647` refusing `false` or zero on either key. The shipped values are in
`default.toml` — `4194304` and `"30s"`, the numbers `nvs_server::bounds` already applies inbound.

**`max_message` reaches the wire; `send_timeout` does not yet.** `openSocket` hands
`transport::upgrade` a `WebSocketConfig` whose cap is the `maxMessage` option, then the directive,
then `DEFAULT_MAX_MESSAGE`, so `tungstenite`'s own 64 MiB default is gone. Nothing writes a frame to
a real peer yet, so the send wait has no write to bound — it is validated and not acted on, and
`tools/directives.py` cannot see that difference (playbook, *Tooling*).

**What is still not real is the conversation.** `receive`, `send`, `sendBytes` and `close` read and
write the *scripted* table, so a socket opened against a real host completes its handshake and then
reads as a peer that said nothing.

## Next group

**Stage 5: the conversation and its bounds** — one file set: `crates/nvs-stdlib/src/http/socket.rs`,
with `crates/nvs-stdlib/src/http/transport.rs` for the framed connection it reads through.

- [ ] **`receive`, `send` and `sendBytes` read and write the held conversation where the table is not
      armed** — `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call` and
      `rule:security/tainted-sources` for the payload that comes back. The branch is the one
      `openSocket` already takes: a socket holding a key in `HELD_AT` reads `Open`'s framed connection
      out of the request's table, one holding `null` reads the scripted peer. `taken` at
      `crates/nvs-stdlib/src/http/socket.rs:492` is the scripted half and stays; the live half is a
      `read()` whose `Ping`, `Pong` and `Close` frames are answered rather than handed to the program.
      `crates/nvs-stdlib/src/http/socket.rs:545` is `receive`, `:571` is `send`.
- [ ] **The bounds and the two closes** —
      `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap` for the
      four waits and ADR 0183 § 7 for the codes. `idle`, `maxDuration` and `ping` are read the way
      `max_message` now is (`crates/nvs-stdlib/src/http.rs:2885` is `cap_of`,
      `crates/nvs-stdlib/src/http.rs:2502` is `judge_bound`); `send_timeout` bounds the write and
      closes the key's last gap. A message past the cap closes with `1009` and the waiting `receive`
      throws naming it; a task that ends closes with `1001`, at
      `crates/nvs-stdlib/src/http/socket.rs:618`.

## Backlog

- `[http.client.socket] send_timeout` is refused when it is not a bound and applied nowhere — the two
  send members are what owe it (docs/decisions/0183.md § 7).
- A second concurrent `receive` on one socket is a `LogicError`, and nothing holds the state to know
  (docs/decisions/0183.md § 7).
- `permessage-deflate`, RFC 8441, reconnecting and a socket handed to another isolate are out of this
  goal on purpose (docs/agent/loop-goal.md § *Standing decisions*).
