# Handoff

## State

**Goal `websocket-client` — a program holds a WebSocket to another server, opened like an outbound
call and closed with the task that opened it.** Stage 1 (goal `outbound-proxy`'s list) is the floor
and passes; stage 2's record, [ADR 0183](../decisions/0183.md), is accepted; stage 3's scripted peer
is on disk and green.

**Stage 4's code is landed and its `.nvst` check is green.** `openSocket` branches on
`ctx.faked_http().is_armed()`: the armed arm keeps the table's `LogicError`, and the unarmed arm
assembles a `transport::Call` and calls `transport::upgrade`, which dials through
`transport::dialled` — one connect implementation, shared with `one` — and runs `tungstenite`'s
`client_with_config` over the connection it left. A non-`101` throws, a `3xx` names its `Location`,
and nothing is pooled. The live conversation is filed in the request's table under the class's new
`held` slot, and `close` sends the close frame and gives the connection back.

**What is not real yet is the conversation.** `receive`, `send` and `sendBytes` still read and write
the *scripted* table, so a socket opened against a real host completes its handshake and then reads
as a peer that said nothing — stage 5's work, with `maxMessage`, `idle`, `maxDuration`, the send wait
and `ping` (judged, never applied) and `close`'s `$code`/`$reason` (accepted, never sent).
`tungstenite`'s own finite defaults are what bound a message until then.

**Spellings this stage fixed, which no later stage re-opens:** `Bag` is one row's flattening of the
shared connection keys plus its scheme roster — `REQUEST_BAG` and `SOCKET_BAG` — and `approved`,
`judge_trust`, `policy_of` and `identity_option` each take one instead of assuming the request's
numbers. The four `TLS_PIN`…`TLS_MIN_VERSION` slot constants are gone: the group is counted off
`TLS_CA` by `Bag::ca()`…`Bag::min_version()`, because `connection_options!` emits it as one run.

## Next group

**Stage 4: the loopback tests, which are what the stage's second check names** — one file set:
`crates/nvs-stdlib/src/http/transport.rs` (its `mod tests`, from line 2772), and
`crates/nvs-stdlib/src/http/socket.rs`.

- [ ] **A WebSocket origin helper, then `socket_handshake_connects_to_the_approved_address` and
      `socket_handshake_carries_the_callers_headers_and_offered_protocols`** —
      `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call`,
      [ADR 0183](../decisions/0183.md) § 4. The helper goes beside
      `crates/nvs-stdlib/src/http/transport.rs:2819` (`origin_raw`) and has to *derive* the accept
      key with `tungstenite::handshake::derive_accept_key` from the request's `Sec-WebSocket-Key`: a
      static `101` cannot carry a valid one, and `tungstenite` checks it. What is under test is
      `crates/nvs-stdlib/src/http/transport.rs:1712` (`upgrade`), and `crates/nvs-stdlib/src/http/transport.rs:2895`
      (`call`) is the `Call` to copy, with its `url` rewritten `ws://`.
- [ ] **`a_101_choosing_a_protocol_not_offered_is_refused` and
      `a_redirect_answering_a_socket_handshake_is_a_runtime_error_naming_the_location`** —
      [ADR 0183](../decisions/0183.md) § 4. The redirect half is already in
      `crates/nvs-stdlib/src/http/transport.rs:1850` (`refused`) and needs only an origin that
      answers `302`. The subprotocol half is judged a layer up, in
      `crates/nvs-stdlib/src/http/socket.rs:489` (`offered`), which no Rust test can reach without a
      compiled call — the cheapest honest shape is to move that judgement into `upgrade`, which is
      already handed the offers, and have the scripted arm at
      `crates/nvs-stdlib/src/http/socket.rs:342` call the same helper.
- [ ] **`wss_socket_completes_over_the_tls_client_under_the_calls_policy` and
      `a_socket_is_opened_through_the_proxy_tunnel_when_one_is_configured`** —
      `rule:security/one-tls-client`, `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`.
      Both helpers already exist: `crates/nvs-stdlib/src/http/transport.rs:4810` (`tls_origin`),
      `crates/nvs-stdlib/src/http/transport.rs:4782` (`trusted`) and the loopback proxy at
      `crates/nvs-stdlib/src/http/transport.rs:2951` (`proxy`), which pumps a TLS flight through
      untouched. The `wss` scheme reaches TLS through `crates/nvs-stdlib/src/http/transport.rs:2476`
      (`parts`).

## Backlog

- Stage 5 — the conversation over a real socket, and every bound applied rather than judged
  ([loop-goal.md](loop-goal.md) § *Stage 5*).
- Stage 6 — flip stage 2's four rules to `shipped` with `guardedBy` filled ([loop-goal.md](loop-goal.md)).
- `[http.client.socket]`'s two directives are not in `crates/nvs-config` yet — ADR 0183 § 8 fixes
  their shipped values, and `crates/nvs-config/tests/tree.rs` is where the block is asserted.
- `.claude/worktrees/gap-zero-restructure/` is an untracked second checkout that doubles every
  `--locate` and `grep` hit in this tree.
