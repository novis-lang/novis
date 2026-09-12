# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections.
Stages 1–5 are on disk; nothing of stages 6–14 is.**
[ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal executes.

`Core\Http\Response` carries six instance rows over three slots
(`crates/nvs-stdlib/src/http.rs:904`): `status`, `text`, `bytes`, `jsonAs<T>`, `header` and `headers`.
**The body slot has three readers and none of them consumes it** — `bytes` asks nothing, `text` asks
whether the octets are UTF-8, `jsonAs<T>` reads them as one JSON document — so any may follow any
other. `jsonAs` is `Core\Json::decode_as` over that slot, and it is on
`registry::WRITTEN_CLASS_MEMBERS`, which is what puts the descriptor, the list flag and an inline
shape's contract ahead of the receiver (`args: [5]`). Its `T` must declare `tainted` on every text
field; both halves of that check now cover this owner — see the playbook bullet for why the class half
and the shape half are keyed differently.

`retry_after` (`crates/nvs-stdlib/src/http/transport.rs:663`) reads both of RFC 9110 § 10.2.3's forms,
on a `429` and a `503` and on nothing else, and a date already past answers `None` so the jittered
backoff stands. The wait is not clamped where it is read — `attempts` already throws where any wait
would end past the deadline.

Nothing is blocked and no design question is open.

## Next group

**Stage 6: streaming, the reader and its bounds** — one file set: `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/registry.rs`.

- [ ] **`Core\Http\Stream` is a class and `Client::stream` is the row that answers one** — a seventh
      `CLIENT` row beside `request` at `crates/nvs-stdlib/src/http.rs:816`, taking a
      `Core\Http\Method` ahead of the URL as `request` does, and a new `CoreClass` beside `RESPONSE`
      at `crates/nvs-stdlib/src/http.rs:904` whose slots pair with its members the same way. The
      reader's shape is `Core\Request::bodyStream`'s (`crates/nvs-stdlib/src/request.rs:314`), and
      `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it` is what
      makes a second read throw. The record says whether `events`, `lines`, `chunks` and `saveTo` are
      four members of one class.
- [ ] **The body runs under `idle` and `maxDuration`, and a stream is retried only before its head** —
      `attempts` at `crates/nvs-stdlib/src/http/transport.rs:300` is where an attempt is decided, so
      it is where "after the first byte of body a failure ends the stream" has to hold;
      `rule:http-server/no-spelling-for-an-unbounded-wait` is why neither key has an unbounded
      spelling and why either on a buffered member is a diagnostic. The cargo tests the stage names
      are `stream_idle_ends_a_silent_reply`, `stream_max_duration_ends_an_endless_reply` and
      `stream_is_retried_before_its_head_and_never_after`.
- [ ] **`events()` parses the WHATWG EventSource format** — a line ends at `\r\n`, `\r` or `\n`;
      `data`, `event` and `id` are read, `retry` is read and ignored, a comment line is skipped, and a
      line and an event are each length-capped beside `REPLY_CEILING` at
      `crates/nvs-stdlib/src/http/transport.rs:74`. Every field is `tainted string`
      (`rule:security/tainted-sources`).

## Backlog

- Parsing `Link`, `Retry-After` for a program, and RFC 9457 problem details are the `nvs/rest`
  package's, not this goal's — `docs/agent/loop-goal.md` § *Standing decisions*.
- The two obsolete HTTP-date forms RFC 9110 § 5.6.7 still lists are not parsed; the reasoning is on
  `http_date` in `crates/nvs-stdlib/src/http/transport.rs`.
