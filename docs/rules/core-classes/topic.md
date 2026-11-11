`Core\Topic` is the only way two persistent connections meet: `subscribe`, `publish` and
`unsubscribe`, runtime-owned, in-process, and reaching across every core. This is the one place the
thread-per-core design is crossed on purpose, and it is a bounded message hand-off rather than shared
state. A published value is graph-copied, so subscribers share nothing with the publisher or with
each other.

**A value is encoded once, and each subscriber decodes it into its own classes.** `publish` writes
the value with the bytes carrier of `rule:classes/graph-copy`, before it looks at who joined, so a
value that cannot cross throws whether or not anyone listens. One buffer goes to every subscriber, on
this core and on every other. Each subscriber's `receive()` decodes it with its own program's
classes, as `Core\Serialize::decode` does. So an object arrives as the class of that name in the
subscriber's program, with every property checked. A subscriber whose program cannot make that class
gets a `ParseError` from `receive()`, naming the topic and the class. That message is off the queue,
the next `receive()` reads the next one, and no other subscriber is affected. **What it spends:** one
encode per publish, however many joined, and one decode per subscriber on that subscriber's own turn
and memory balance. The buffer is the process's bytes, held once until the last queue drops it. Each
queue counts its full length against its subscriber's memory limit, which over-counts a shared
buffer, the safe direction.

One carrier serves every path, so a subscriber on this core and one on another core get the same
classes and the same errors.

**Declined:**
- **A live copy per subscriber.** The live carrier keeps the sender's class descriptor, so a
  program's class would reach a subscriber on another core and never one on the same core.
- **Dropping a message a subscriber cannot decode, silently.** An old connection would stay open and
  miss messages. After a deploy, a connection that kept its old unit ends on the first message whose
  class changed, unless it catches, and the client reconnects into the new unit.

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

**A topic name belongs to one application.** Two `[[app]]` blocks served by one process may use the
same name, and a publish in one never reaches a subscriber of the other, on this core or on any
other. The table and the hand-off between cores key a topic by the application's path, a NUL byte
and the name: the identity the memory cache uses, without the generation, because a connection
outlives a reload. The program still reads the bare name in `Message->topic`, and the 256-byte bound
is on that name. `nvs run --events` publishes under the configuration of the program it feeds. **What
it spends:** one key built per member call, and the path's bytes once more in each table row.

There is no way to broadcast without it and no way for one connection to reach another directly,
which is what keeps a connection an isolate rather than a thread with a socket. Cross-machine fan-out
is not the runtime's: a fleet bridges topics to a broker in application code.

**Declined:**
- **Building it on the shared cache.** The cache is deliberately lossy
  (`rule:concurrency/cross-request-state-is-explicit`), which is right for a cache and wrong for a
  message a subscriber is waiting on, and one mechanism with two contracts would make both harder to
  reason about.
- **A bus between machines.** It would mean owning a distributed system's failure modes, which the
  placement tests leave to a dependency (`rule:core-api/tier-placement`).
