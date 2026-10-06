`Core\Net` is the whole socket surface: one class over three transports, TCP, UDP and Unix-domain
sockets. A program reaches them through
five entry points — an outbound TCP connection, a listening TCP socket, a bound UDP socket, an
outbound Unix-domain connection and a listening Unix-domain socket — and accepting is a member on the
listener rather than a sixth way in.

**No entry point decides between transports by reading its argument.** A host and a socket path are
separate members taking separately-typed arguments, so `rule:security/a-path-is-not-a-url`'s refusal —
no member dispatches on the textual content of a path — holds by construction rather than by a check.
That is why the surface is five members and not two: a single connect taking `"unix://…"` or
`"tcp://…"` would be one function distinguished by a prefix, and the separate member is the security
property.

Every one of those sockets parks on the runtime's own reactor. `crates/nvs-host/src/net.rs` is the
contract they share — a `Read` and a `Write` that hand the core back instead of blocking it — and
`crates/nvs-host/src/reactor.rs` is its readiness half. A second event loop is never the answer: it
would be a second poll structure whose fairness, shutdown, deadline and drain semantics must be made
to agree with the reactor's by hand, and whose disagreements appear only under load, which is what
`rule:concurrency/one-scheduler` refuses for the scheduler on the same ground. A shape that cannot be
expressed over the reactor is cut, not given a loop of its own.

The connected transports answer a `Read` and a `Write` shaped like every other stream in the language.
A datagram socket does not, because it has no stream to read: it sends and receives whole messages,
addressed one at a time.
