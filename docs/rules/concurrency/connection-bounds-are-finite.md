Every bound on an open connection is finite before anything is configured: connections per process,
maximum frame size, maximum message size, idle timeout, total lifetime, send timeout and subscriber
queue depth. A connection is the one thing a server holds that no request ever ends, so a bound left
to a later configuration pass is a bound absent on every deployment that did not know to write it.

**A deployment moves a bound and cannot remove one.** The `[server.connection]` block writes over the
ones a connection is framed inside — how many the process holds open, the frame and message caps, the
idle and lifetime windows, the send wait — and every key it leaves out keeps the shipped number, so a
partly-written block is a decision per key rather than one decision about the table. Neither `false`,
which removes a ceiling everywhere else in the configuration, nor zero, which is that value from the
other side, is a value any of those keys has: both are refused while the file is being read, before a
listener exists. A connection is framed inside the bounds of the snapshot published when it is
accepted, so a reload reaches the next connection, and a connection already open keeps what it was
accepted under.

**The lifetime is the time bound a connection has, and `[limits] wall_time` is not one.** The
`wall_time` of the request that upgraded ends that request and never the connection
(`rule:concurrency/a-connection-is-a-root-isolate`), so the security checklist's `wall_time` and a
live page work together. A connection's tree still keeps `memory`, and `cpu_time` for each turn
(`rule:concurrency/a-connection-is-a-root-isolate`).

Each bound ends in a **defined close** rather than a reset, and the connection's own loop takes it:
`receive()` answers `null` and the peer is told which bound it met. A connection that exceeds its
memory, CPU or lifetime budget is reported as that and never as an out-of-memory.

**Both doors are bounded, and not by the same bounds.** An event stream has no codec and no peer
speaking, so what holds it is a lifetime, a drain period and one event's size — plus two numbers no
operator writes, and one field it must leave alone. **It takes a `max_open` place like a WebSocket**:
it holds a root isolate and a task stack as long as one, so it counts against the same number. The
door takes the place before the isolate starts and the body gives it back when it is dropped, and with
no place free the request is answered `503` with `Retry-After` and no isolate starts. **What it
spends:** one relaxed add and sub and one word per stream.

- **The keep-alive is derived rather than configured.** What would close a quiet event stream is the
  wait a connection writes responses under, so the interval that keeps one open is read off that same
  wait — half of it, floored at a second wherever the wait leaves room for one — and is therefore a
  number nobody configures twice. A beat equal to the wait is a race and not a bound: the stream
  writes its keep-alive at the instant it is already being closed for not having written one.
- **The reconnection hint is drawn per stream**, spread by up to a third either side of the number
  the server holds. A constant tells every client of a drained instance to come back at the same
  moment, so the instance replacing it takes the whole fleet in one arrival. The spread is narrow
  rather than full jitter from zero: this is the single gap before a client returns, not a backoff
  against a contended resource, and a client drawn near zero reconnects into the restart it was told
  to wait out.
- **The idle timeout is unarmed on this door, and that is the bound's design.** It closes a
  connection whose *peer* stopped speaking, and an event stream's peer never speaks — the hand-over
  took nothing from it (`rule:concurrency/two-doors-one-isolate`) and there is no frame it could
  send — so arming it would close every healthy stream at the first quiet window. An absent bound is
  the one thing a list of bounds cannot show by listing, so it is stated here rather than left to be
  noticed as a gap.

Connections *per tenant* is deliberately not a bound of the server's. The upgrade is an ordinary HTTP
request, so a per-tenant ceiling is the rate limit the route already declares
(`rule:core-classes/ratelimit-two-members`); a second ceiling here would be a second policy over one
request.

**Only a message restarts the idle timeout.** `receive` fixes the idle instant when it starts to wait
and keeps it across pings, pongs and empty fragments, so a peer that sends only those is closed with
the idle close when the instant passes. A message that never ends meets the instant at the stream's
streak check, within sixteen reads. The lifetime is compared where each `receive` and each `send`
starts, so frames that keep arriving cannot carry a connection past it. An application that wants a
longer-lived connection sends a message. **What it spends:** one clock read per control frame and one
per operation. **Declined:** a count of control frames per message, which would be a new limit.
