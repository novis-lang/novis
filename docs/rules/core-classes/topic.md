`Core\Topic` is the only way two persistent connections meet: `subscribe`, `publish` and
`unsubscribe`, runtime-owned, in-process, and reaching across every core. This is the one place the
thread-per-core design is crossed on purpose, and it is a bounded message hand-off rather than shared
state. A published value is graph-copied, so subscribers share nothing with the publisher or with
each other.

**A slow subscriber is closed, never tolerated.** Each subscriber has a bounded queue, drained by its
own connection and by nothing else; on overflow *that subscriber's connection* is closed with a
defined code and a metric increments. The publisher is never blocked and no queue grows without
bound — a fan-out to ten thousand clients must not become a way for one of them to stall the other
nine thousand nine hundred and ninety-nine.

A topic name **refuses `tainted`**, for the reason a metric label does: a name derived from user input
is how one tenant subscribes to another's stream. All three members refuse it in the same words, and
before the connection is consulted. A `secret` may never be published.

**The topic table has a ceiling per connection.** Its bytes are the process's, so no connection's
memory limit bounds them. Instead a connection is in at most 1024 topics at once, and a name is at
most 256 bytes; past either, the member throws a `LogicError` and the table is unchanged. A row whose
subscribers have all ended is swept by a later insert even when nobody names it again, so the table
is O(live connections) and never O(connections served).

It is **not** built on the shared cache, which is deliberately lossy — right for a cache and wrong
for a message a subscriber is waiting on. Cross-machine fan-out is not the runtime's: a fleet bridges
topics to a broker in application code.
