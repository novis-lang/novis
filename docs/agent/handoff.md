# Handoff

## State

**Goal `websocket-client` — stage 5, the conversation.** Stages 1–4 are closed: the handshake is live
over plaintext `ws`, over `wss` through the process's one outbound TLS client, and through the
operator's `CONNECT` tunnel. Stage 5's configuration half landed earlier — `[http.client.socket]` at
`crates/nvs-config/src/tree.rs:679`, with `max_message` reaching the framing.

**The conversation is now live.** `receive`, `send` and `sendBytes` take the branch `openSocket`
already took: a socket holding a key in `HELD_AT` reads and writes `Open`'s framed connection out of
the request's table, one holding `null` reads and records the scripted peer. A ping or a pong is
answered by the codec and never handed to the program, and a close — the peer's or the connection's —
reads as the `null` that already means the conversation is over.

**Three of the four bounds are armed**, because the handshake left the stream bound by the *opening
call's* deadline and that instant says nothing about how long a conversation may wait. `Open` carries
`idle`, `until` (`maxDuration`, measured from where the opening call began) and the send wait, and
each member arms the wait it is about to take; an expired one throws `TimeoutError`
(`crates/nvs-stdlib/src/http/socket.rs:713` and `:724` are the two spellings). `DEFAULT_SEND_TIMEOUT`
is new in `crates/nvs-stdlib/src/http.rs`.

**What is still not real**: `ping`, the `1009` close on a message past the cap, the `1001` close when
the task ends, and `close`'s own `code` and `reason`, which the member takes and drops. Nothing at the
Rust level reaches the live half yet: `crates/nvs-stdlib/src/http/transport.rs:2943`'s loopback origin
holds its connection and says nothing, so a peer that talks is the harness stage 5's six named tests
are waiting on.

**The pack's `[context] rules` names neither socket rule stage 5 is about** —
`http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap` and
`http-server/an-outbound-socket-belongs-to-the-task-that-opened-it` — and both were fetched by hand.

## Next group

**Stage 5: the bounds, the two closes and the peer that talks** — one file set:
`crates/nvs-stdlib/src/http/socket.rs`, with `crates/nvs-stdlib/src/http/transport.rs`'s test module
for the loopback peer the assertions need.

- [ ] **A loopback peer that talks** — the harness stage 5's six `cargo test -p nvs-stdlib` names in
      `docs/agent/loop-goal.toml:10011` are all waiting on: frames on demand, a silence, a ping, and a
      message past the cap. `crates/nvs-stdlib/src/http/transport.rs:2943` is `socket_origin`, which
      answers one handshake and then holds the connection saying nothing.
- [ ] **`ping`, and `close`'s code and reason** —
      `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap` for the
      ping being off unless a program sets it, ADR 0183 § 7 for the codes. The read loop is
      `crates/nvs-stdlib/src/http/socket.rs:632`, and `crates/nvs-stdlib/src/http/socket.rs:846` is
      the `close` that takes two arguments and writes neither.
- [ ] **The `1009` and the `1001`** — a message past `maxMessage` closes with `1009` and the waiting
      `receive` throws naming it (`crates/nvs-stdlib/src/http/socket.rs:692` is `failed`, where the
      codec's capacity error arrives); a task that ends closes with `1001`
      (`rule:http-server/an-outbound-socket-belongs-to-the-task-that-opened-it`), which is the request
      giving the connection back at `crates/nvs-stdlib/src/http/socket.rs:846`.
- [ ] **Two `receive`s waiting on one socket is a `LogicError`** — ADR 0183 § 7. Nothing holds the
      state to know; `crates/nvs-stdlib/src/http/socket.rs:613` is `open_at`, the one door to the held
      conversation, so the mark belongs on `Open`.

## Backlog

- `Core\Http\Socket`'s module is past 870 lines, which is where
  `docs/agent/playbook.md` § *Splitting a file that got too big* starts to apply.
- `permessage-deflate`, RFC 8441, reconnecting and a socket handed to another isolate are out of this
  goal on purpose (docs/agent/loop-goal.md § *Standing decisions*).
