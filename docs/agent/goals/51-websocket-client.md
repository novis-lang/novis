---
milestone: M8
---
# Loop goal 51 — a program holds a WebSocket to another server, opened like an outbound call and closed with the task that opened it

A program can open a WebSocket to another server — a realtime API, a chat platform's gateway, another
Novis instance — and hold a conversation over it. It is opened through the door every outbound call
passes: the URL is a sink, the host is pinned and checked, the TLS policy, a client identity and the
operator's proxy all apply, and a test answers it from `Core\Test`'s table instead of a listener. What
comes back is read as the `Core\Socket\Message` a server-side connection already answers with, every
wait on it is bounded, and it never outlives the task that opened it.

## Why here

After goal `outbound-proxy`, on the user's call, because the opening handshake is an outbound call and
everything the three goals before it build governs it: goal `http-client`'s pin over every approved
address, TLS policy, client identity, `idle` and `maxDuration`, and table in `Core\Test`, and goal
`outbound-proxy`'s tunnel, which a socket opened behind a proxy must go through. Before goal `gap-zero`
for that goal's standing reason — a register is emptied after everything that adds to it has run.

What it needs already built: the server's half — `tungstenite` over the parking stream with no adapter
(`Cargo.toml:149-167`), `Core\Socket`'s `receive`, `send` and `sendBytes`
(`crates/nvs-stdlib/src/socket.rs:256-296`) and `Core\Socket\Message` (`:446-517`), whose `topic` and
`value` already answer `null` for a peer frame; the URL half's scheme check
(`crates/nvs-stdlib/src/http.rs:262-263`); and, once goal `http-client` lands, its transport's connect,
TLS hand-off and outbound table.

## Stage 0 — the catch-up

Sentences on disk this goal makes wrong. Each is corrected in the stage that makes it wrong. Re-grep before
editing: these are anchors, and files move.

- `crates/nvs-stdlib/src/http.rs:262-263` — the scheme roster is `http` and `https`. Stage 4 adds `ws` and
  `wss` for the one row that opens a socket, and `rule:http-server/allow-url-pins-the-address`'s
  "refuses a scheme outside the grant" moves with it by the record's `changes.modifies`.
- `Cargo.toml:162-166` — the `tungstenite` block says why a `wss://` *listener* needs no TLS feature.
  Stage 4 adds the client half, which needs none either: the transport hands `tungstenite` a stream
  `nvs_host::tls` already decrypted.
- `crates/nvs-stdlib/src/socket.rs:446-473` — `Core\Socket\Message` is "the one shape both of `receive`'s
  sources answer in". Stage 5 makes it the shape an outbound socket answers in too, and rewrites that
  doc whole.

## Stage 1 — the floor

Goal `outbound-proxy`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the record

One new record, and no other number. Its body is § *Standing decisions* below, argued. It creates four
rules, all `designed`:

| Rule | Says |
|---|---|
| `http-server/an-outbound-socket-is-opened-like-an-outbound-call` | the URL sink, the pin, every grant, the TLS policy, a client identity and the proxy apply to the opening handshake as to any call; `ws` and `wss` are its schemes; a socket is never pooled |
| `http-server/an-outbound-socket-belongs-to-the-task-that-opened-it` | charged to that task's budget, closed with `1001` when it ends, never a root isolate and never handed to another isolate |
| `http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap` | `idle`, `maxDuration`, `maxMessage` and the send wait, each inherited from `[http.client]` when omitted, none with an unbounded spelling |
| `testing/an-outbound-socket-is-answered-by-a-scripted-peer` | `Core\Test`'s table answers the handshake and plays the frames it was given, and records what the program sent |

It modifies `http-server/allow-url-pins-the-address` (two schemes join, for one row). The record fixes
the spellings the surface leaves open — the row's name and the socket class's — under
`rule:core-api/verb-lexicon`.

## Stage 3 — the keystone: a socket answered by a scripted peer

`crates/nvs-stdlib/src/test.rs`, beside goal `http-client`'s outbound table. A test registers a URL with
the frames a peer should send; a matched socket completes its handshake without connecting, receives
those frames in order and then a close, and every frame the program sent is recorded for the test to
read back. An unmatched socket throws naming the URL, as an unmatched call does.

Every later `.nvst` case is written against it. What it cannot reach — masking, fragmentation, the close
handshake, a ping — is proved by Rust tests against a loopback peer, as `transport.rs`'s tests are.

## Stage 4 — the handshake

`crates/nvs-stdlib/src/http.rs` and `crates/nvs-stdlib/src/http/transport.rs`.

- **A row on `Core\Http\Client`** takes `string|Core\Http\Target $url` and the options bag, and answers
  the socket once the `101` has arrived. `ws` and `wss` join the scheme roster for this row alone: the
  request rows refuse them and this row refuses `http` and `https`, so what a URL is for is written in
  it. `allowUrl` pins either exactly as it pins `http` and `https`.
- **The handshake is an outbound call**: the same pin and fallback across addresses, the same
  `connectTimeout`, a `deadline` that ends at the `101`, the same TLS policy options and grants, a
  client identity, the proxy's tunnel, and `headers` — a `secret` value among them, as goal
  `http-client`'s stage 4 allows. A redirect is not followed: a `3xx` answering an upgrade is a
  `RuntimeError` naming the `Location`.
- **`protocols?: array<string>`** is `Sec-WebSocket-Protocol`; a `101` choosing one that was not offered
  is refused, and the socket reports the one chosen.
- **`tungstenite`'s client half, over the stream the transport already has** — the parking stream, or
  `nvs_host::tls`'s decrypted one — with no adapter and no new dependency, for the reason the server's
  half gives (`Cargo.toml:155-160`). Masking, fragmentation, UTF-8 validation and the close handshake
  are that crate's, never this repository's.
- **Never pooled.** A socket consumes its connection, and nothing goes back.

## Stage 5 — the conversation and its bounds

A new module under `crates/nvs-stdlib/src/http/`, and `crates/nvs-stdlib/src/socket.rs` for the message.

- **`receive()`, `send(string)`, `sendBytes(bytes)`, `close(?uint $code, ?string $reason)` and
  `protocol()`**, the first three in `Core\Socket`'s shapes and for its reasons (`socket.rs:266-273`: two
  classified parameters, not a union). `receive` answers `?Core\Socket\Message` — `text` or `bytes`
  filled and `tainted`, `topic` and `value` `null` — and `null` once the peer has closed.
- **The bounds**: `idle`, the longest silence; `maxDuration`, the socket's whole life; `maxMessage`, the
  largest message after reassembly, past which the socket is closed with `1009`; and the send wait.
  Each inherits `[http.client]` when omitted — goal `http-client`'s `idle` and `max_duration`, and a new
  `[http.client.socket]` block for `max_message` and `send_timeout` — and none has an unbounded
  spelling. `ping?: Duration` sends a ping after that much silence, so `idle` ends a dead peer rather
  than a quiet one; unset sends none, and a peer's ping is always answered.
- **It belongs to the task that opened it.** It is charged to that task's budget and closed with `1001`
  when the task ends, so a request's `wall_time` bounds it too. It never crosses an isolate boundary —
  `rule:classes/graph-copy` refuses it as it refuses any resource. Two `receive`s waiting at once on one
  socket is a `LogicError`.
- **`close`** sends the close frame and waits for the peer's under the send wait; a peer that never
  answers is closed anyway, and that is not an error.

## Stage 6 — the rulebook

Flip stage 2's four rules to `shipped`, with `guardedBy` filled from this goal's cases and tests, and
`python tools/rules.py --render`.

## Standing decisions

- **Settled with the user**: an outbound WebSocket client is in, as its own goal after goal
  `outbound-proxy`. It is a transport rather than an authentication flow, so
  `rule:security/protocol-admission-test`'s boundary does not keep it out of `Core`.
- **Opened like a call, answered like a server-side connection.** The door, the pin, the grants, the TLS
  policy, the identity and the proxy are the client's; the message shape is `Core\Socket`'s. A second
  message class for the same RFC 6455 frame is the copy that disagrees.
- **Not a root isolate.** A server-side connection is one because it outlives the request that upgraded
  it (`rule:concurrency/a-connection-is-a-root-isolate`); a client socket is a value the program opened
  and holds, so it lives and dies with that program's task and memory stays O(in-flight). A program
  that wants a socket to outlive a request opens it in whatever outlives the request — a command, a
  queue job, a spawned script. The fallback, if a socket cannot be tied to its task's end, is to refuse
  opening one in a task that has no end.
- **`ws` is allowed as plain `http` is.** There is no downgrade question, because a socket follows no
  redirect.
- **Every received payload is `tainted`**; a sent frame is not a sink (`socket.rs:277-280`).
- **Config**: `[http.client.socket]` — `max_message` and `send_timeout` — is `Runtime`, as
  `[http.client] deadline` is, because each bounds one call. The record fixes their shipped values, and a
  default that is unbounded is a defect (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`).
- **What it spends**: per open socket, one connection — a socket and, for `wss`, a TLS session —
  `tungstenite`'s read and write buffers, and at most `maxMessage` for a message being reassembled, all
  charged to the task that opened it and released when it closes. That is O(open sockets), and a socket
  never outlives its task, so it is O(in-flight). Nothing per core and nothing per process.
- **ADR slots**: the one record of stage 2.
- **Not this goal**: `permessage-deflate`, whose context takeover holds a window per socket and whose
  inflate is a bomb surface of its own; WebSocket over HTTP/2 (RFC 8441); reconnecting, which is the
  program's; a socket handed to another isolate; subprotocol libraries — STOMP, Socket.IO, GraphQL over
  WebSocket — which are packages; any change to the server's half. A session that finds one on its path
  writes it to the handoff's `## Backlog`.
