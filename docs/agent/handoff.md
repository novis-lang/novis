# Handoff

## State

**Goal `websocket-client` — a program holds a WebSocket to another server, opened like an outbound call and
closed with the task that opened it.** Stage 1 (goal `outbound-proxy`'s whole list) is the floor and passes.

**Stage 2 is done: [ADR 0183](../decisions/0183.md) is on disk, accepted, with its four rules and the
amendment to `rule:http-server/allow-url-pins-the-address` in the same commit.** `rules.py --check`,
`records.py --check` and `check-links.py` are all clean and the chapters are rendered. The record is the
only ADR slot this goal has; every later stage cites it rather than deciding anything new.

**The spellings it fixed, which no later stage re-opens:** the row is
`Core\Http\Client::openSocket(string|Core\Http\Target $url, ...$options): Core\Http\Socket`; the class is
`Core\Http\Socket` and the message stays `Core\Socket\Message`; the test rows are `Core\Test::answerSocket`
and `Core\Test::sentSocket`; the bounds are `idle`, `maxDuration`, `maxMessage`, `sendTimeout` and `ping`,
with `[http.client.socket] max_message = 4194304` and `send_timeout = "30s"` shipped, both `Runtime`.

Nothing of stages 3–6 has landed: no Rust, no `.nvst`, no registry row.

## Next group

**Stage 3: the surface a scripted peer can answer** — one file set: `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/test.rs`, `crates/nvs-stdlib/src/socket.rs`.

- [ ] **Register `Core\Http\Socket` and the `openSocket` row**, and add the socket option group to the
      bag macro — `rule:http-server/an-outbound-socket-is-opened-like-an-outbound-call` and
      `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`. The rows are
      `crates/nvs-stdlib/src/http.rs:1291`, the bag macro `crates/nvs-stdlib/src/http.rs:670`, and the two
      bounds the `stream` row already declares `crates/nvs-stdlib/src/http.rs:874`. The class shape to copy
      is `crates/nvs-stdlib/src/socket.rs:256` — two classified parameters, never a `string|bytes` union.
- [ ] **Widen the scheme roster to four, one row each** — `rule:http-server/allow-url-pins-the-address`.
      `crates/nvs-stdlib/src/http.rs:436-443` is the check and its message; the row that took the URL is
      what decides which two of the four it serves, so the refusal moves from one roster to a per-row one.
- [ ] **`Core\Test::answerSocket` and `sentSocket`** —
      `rule:testing/an-outbound-socket-is-answered-by-a-scripted-peer`. Beside `answerHttp` at
      `crates/nvs-stdlib/src/test.rs:434` and `sentHttp` at `crates/nvs-stdlib/src/test.rs:455`, whose
      table this reuses rather than starting a second one.

## Backlog

- `crates/nvs-stdlib/src/http.rs:44` and `:104` say "the client's five rows" and that `patch` is "not here
  yet", and both are wrong today, against the no-counts half of [conventions.md](conventions.md) § *A code
  comment*. Stage 4 rewrites that header when it adds a row; fix it there, as a count-free sentence.
- `crates/nvs-stdlib/src/socket.rs:446-473` calls `Core\Socket\Message` "the one shape both of `receive`'s
  sources answer in" — stage 5 rewrites it whole as the shape both *directions* answer in.
- Stage 6 flips the four new rules from `designed` to `shipped` and fills their `guardedBy`; nothing else
  may.
- `permessage-deflate`, RFC 8441 and reconnecting are out of this goal by
  [0183](../decisions/0183.md) § *Alternatives rejected*, which names what a later record would owe.
