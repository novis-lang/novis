# Handoff

## State

**Goal `websocket-client` — a program holds a WebSocket to another server, opened like an outbound
call and closed with the task that opened it.** Stage 1 (goal `outbound-proxy`'s list) is the floor
and passes; stage 2's record, [ADR 0183](../decisions/0183.md), is accepted; stage 3's scripted peer
is on disk and green.

**Stage 4 is closed.** All six tests its `cargo-named` check pins pass, and so do the two `.nvst`
cases of its `nvs-suite` check — the whole conformance tree is green. The handshake is live over
every transport it has: plaintext `ws`, `wss` through the process's one outbound TLS client under
the policy the `Call` carries, and either of those through the operator's `CONNECT` tunnel, with a
bypassed host dialled directly.

**`openSocket` branches on `ctx.faked_http().is_armed()`** — the armed arm keeps the table's
`LogicError`, the unarmed arm assembles a `transport::Call` and calls `transport::upgrade`, which
dials through `transport::dialled`, the one connect implementation `one` also uses. Nothing is
pooled, the live conversation is filed in the request's table under the class's `held` slot, and
`close` gives the connection back. The subprotocol judgement is `transport::settled`, which both
arms pass through.

**What is not real yet is the conversation.** `receive`, `send`, `sendBytes` and `close` still read
and write the *scripted* table, so a socket opened against a real host completes its handshake and
then reads as a peer that said nothing. `[http.client.socket]` does not exist in
`crates/nvs-config/src/directive.rs` at all, so neither bound has a value to be read from.

## Next group

**Stage 5: the conversation and its bounds** — one file set: `crates/nvs-stdlib/src/http/socket.rs`,
with `crates/nvs-config/src/directive.rs` for the block the bounds are read from.

- [ ] **`[http.client.socket]` — `max_message` and `send_timeout`, both `Runtime` and both bounded** —
      `rule:http-server/no-spelling-for-an-unbounded-wait`,
      `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`, and the goal's
      § *Standing decisions* for why the class is `Runtime` rather than `System`.
      `crates/nvs-config/src/directive.rs:127` is where the `[http.client]` rows sit, beside the two
      exceptions whose comments say what makes a key `System`. The check that closes it is
      `http_client_socket_max_message_and_send_timeout_are_bounded_and_runtime_class`, in
      `nvs-config`; the template and the directive sweep both count the new leaf keys, so the shipped
      default goes in with the row.
- [ ] **`receive`, `send` and `sendBytes` read and write the held conversation where the table is not
      armed** — [ADR 0183](../decisions/0183.md) § 5, and the goal's § *Standing decisions* for every
      received payload being `tainted`. `crates/nvs-stdlib/src/http/socket.rs:536` (`receive`),
      `crates/nvs-stdlib/src/http/socket.rs:562` (`send`) and
      `crates/nvs-stdlib/src/http/socket.rs:581` (`sendBytes`) are the three bodies; the live socket
      is the `held` slot at `crates/nvs-stdlib/src/http/socket.rs:322`, and
      `crates/nvs-stdlib/src/http/socket.rs:302` is the branch both arms already pass through.
- [ ] **The bounds and the two closes** — `rule:http-server/no-spelling-for-an-unbounded-wait` for the
      waits, [ADR 0183](../decisions/0183.md) § 5 for `ping` (judged, never applied) and for `close`'s
      `$code`/`$reason`. `crates/nvs-stdlib/src/http/socket.rs:609` (`close`). The six tests stage 5's
      `cargo-named` check names are the shape: `socket_idle_ends_a_silent_peer`,
      `socket_ping_keeps_a_quiet_live_peer_open`, `socket_max_duration_ends_an_endless_conversation`,
      `a_message_past_max_message_closes_the_socket_with_1009`,
      `a_socket_is_closed_with_1001_when_its_task_ends` and
      `two_receives_waiting_on_one_socket_is_a_logic_error`.

## Backlog

- Stage 5's two `.nvst` cases are not on disk yet — `docs/agent/loop-goal.toml:9993` names them.
- Stage 6 is the rulebook, and stage 7 the reference surface — `docs/agent/loop-goal.md`.
- `permessage-deflate`, RFC 8441, reconnection and the subprotocol libraries are out of this goal on
  purpose — `docs/agent/loop-goal.md` § *Standing decisions*.
