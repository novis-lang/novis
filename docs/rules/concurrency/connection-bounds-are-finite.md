Every bound on an open connection is finite before anything is configured: connections per process,
maximum frame size, maximum message size, idle timeout, total lifetime, send timeout and subscriber
queue depth. A connection is the one thing a server holds that no request ever ends, so a bound left
to a later configuration pass is a bound absent on every deployment that did not know to write it.

Each bound ends in a **defined close** rather than a reset, and the connection's own loop takes it:
`receive()` answers `null` and the peer is told which bound it met. A connection that exceeds its
memory, CPU or lifetime budget is reported as that and never as an out-of-memory.

Connections *per tenant* is deliberately not a bound of the server's. The upgrade is an ordinary HTTP
request, so a per-tenant ceiling is the rate limit the route already declares
(`rule:core-classes/ratelimit-two-members`); a second ceiling here would be a second policy over one
request.

An application that wants a longer-lived connection sends anything at all — a ping is a frame, and
the loop never sees one.
