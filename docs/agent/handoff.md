# Handoff

## State

**Goal 6, M7 — ADR 0083 § 1's connection is framed end to end.** A request that filled the socket
slot is answered RFC 6455's `101` in place of its own response; `crates/nvs-server/src/serve.rs`
takes the descriptor back through `http1::Connection::into_parts`,
`crates/nvs-server/src/socket.rs` puts `tungstenite` over it, and the connection's root isolate
starts with that peer already on its context.
`a_connection_isolate_reads_and_writes_frames_over_the_upgraded_socket` sends a real client frame
and reads the isolate's answer.

**The socket crosses as a trait object.** `nvs_runtime::PeerSocket` is the seam — two payload kinds,
no control frames, `Ok(None)` for a peer that closed — because `Ctx` is underneath both `NvsTcp` and
the codec. `nvs_host::Isolate::over_socket` moves one onto the child context and `Ctx::peer()` reads
it back; `None` everywhere else is what will make `Core\Socket::current()` a refusal outside a
connection rather than a rule to remember.

**The offer narrowed, and the door's own docs are its home.** `hyper` framing an upgrade answers
"can this connection be taken over"; the door now also requires a `Sec-WebSocket-Key` and version 13
before offering § 1's slot, so a bare `Connection: Upgrade` gets no slot and is answered as the
ordinary request it is.

**The goal's failing acceptance check is unchanged** and its three remaining `-p nvs-stdlib` tests
all need § 3's members. The framing they were waiting behind is now there, so they are writable.

## Next group

**§ 3's loop, read from inside the isolate.** All three share `crates/nvs-stdlib/src/socket.rs` and
`crates/nvs-runtime/src/peer.rs`; the first has to land before either of the others, because
`receive()` has nowhere to be called from until `current()` answers.

- [ ] **`Core\Socket::current()` and `Socket\Message`** — ADR 0083 § 3's two shapes the loop reads.
      The row and its card go in `crates/nvs-stdlib/src/socket.rs:198`'s `CLASS`, the arm in
      `crates/nvs-stdlib/src/socket.rs:273`; the peer it answers from is
      `crates/nvs-runtime/src/ctx/isolate.rs:45`, and a message's payload is
      `crates/nvs-runtime/src/peer.rs:47` with § 4's topic name still to come beside it.
- [ ] **`receive()`, and `null` when the peer closed** — ADR 0083 § 3. `Ok(None)` at
      `crates/nvs-runtime/src/peer.rs:103` is the `null`; the member is a row at
      `crates/nvs-stdlib/src/socket.rs:198` over `crates/nvs-server/src/socket.rs:156`. The **topic**
      half of the one wait needs § 4's bus, which is unbuilt — land the peer half, and say in the
      handoff that `receive_answers_a_peer_frame_and_a_topic_delivery_from_one_wait` is still owed
      its second source.
- [ ] **`send()`'s timeout** — ADR 0083 § 3 with ADR 0074's finite wait.
      `crates/nvs-server/src/socket.rs:179` writes with no deadline armed; arming one on the stream
      before the write (`nvs_host::NvsStream::set_deadline`, as `crates/nvs-server/src/io.rs:203`
      does) is the whole of it, and the wait comes from `nvs_config::server::Waits`.

## Backlog
- § 1's `[limits] idle` arms nothing for a connection — `crates/nvs-server/src/socket.rs` § *What is
  not here yet*.
- `Core\Topic` (§ 4) is unregistered — `crates/nvs-stdlib/src/socket.rs`'s module doc.
- § 5's `200 text/event-stream` is still unwritten — `crates/nvs-stdlib/src/sse.rs:47`.
- `tungstenite`'s input buffer is 128 KiB per open connection, taken as the default — worth a number
  under a many-connection benchmark; `crates/nvs-server/src/socket.rs` § *What it spends*.
