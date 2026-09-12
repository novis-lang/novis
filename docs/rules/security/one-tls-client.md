There is one TLS client in the tree, with one answer to "whose certificates do you believe" and one
place a configured anchor bundle plugs into. A driver that needs a handshake tunnelled inside its own
framing reaches that client over a **generic transport** — an ordinary read/write adapter — rather
than building a second session of its own.

A second TLS session inside a driver crate would be a second answer to a question already decided at
length, and the failure mode is not a compile error: it is one client verifying peers strictly and
another not. Verification is strict by default, and relaxed only where a `capabilities.tls` grant names
the host and the call asks for it (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`) — which
a caller reaches by handing in a policy value the module builds a session from, never a session of its
own. The plaintext phase of a connection is only ever the upgrade request itself.

What does not generalise is what belongs to the socket rather than to the session — the deadline and
the peer address — so there is still one clock, on the thing that waits.
