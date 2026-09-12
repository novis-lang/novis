`deadline` covers a streamed call's connection and its head and ends there; the body runs under two
further `Duration`s, `idle` — the longest silence allowed — and `maxDuration` — the longest the body may
take at all. Both inherit `[http.client] idle` and `max_duration` when a call omits them, and neither has
an unbounded spelling, so a stream has no more way to say *forever* than a buffered call does
(`rule:http-server/no-spelling-for-an-unbounded-wait`). Either key on a buffered member is a compile-time
diagnostic.

Two bounds rather than one because they catch different failures. A server that stops sending is caught
by `idle` in seconds. A server that dribbles one byte per second forever passes every idle check ever
written, and only a lifetime ends it — which is the case a single timeout on a streaming client always
misses.

`Client::stream(Core\Http\Method $method, $url, {…})` answers a `Core\Http\Stream` once the head has
arrived, with `status()`, `header()`, `headers()` and `tls()` readable and the body not yet read. **The
body is then read one way, once** — `events()`, `lines()`, `chunks()` or `saveTo($path, $max)` under
`fs.write`, whose bound is required for `rule:core-classes/io-write-stream`'s reason — and a second read
throws, the same division the inbound half already makes between readers that share a body and readers
that consume it (`rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`).
Every piece a reader yields is `tainted`, a line and one event's accumulated `data` are each
length-capped as a constant rather than a directive, and **a stream is retried only before its head**:
after the first byte of body a failure ends the stream, because the caller has already seen a prefix of
one answer and re-requesting would hand it the prefix of another.
