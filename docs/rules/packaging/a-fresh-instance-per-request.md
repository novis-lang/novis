Each request that calls an extension gets a **pristine instance** of it, created lazily on first use.
Nothing a guest wrote into a global, a static or its linear memory is there on the next request, and a
test that stores state in one instance must not be able to observe it from the next. No extension
keeps state for a worker's lifetime.

Instantiation costs about 8 µs with the pooling allocator, paid only for the extensions a request
actually calls. A typical request touches one to three, so the realistic cost is 8–23 µs against a
request budget measured in milliseconds — and the figure is environment-sensitive, so quote it as a
range.

This is also the property that decides tier placement for anything whose defining feature is state
outliving a request. A connection pool, a bound directory session, a broker consumer cannot be a guest,
because a guest re-instantiated per request loses them every time; such a client is Native or nothing
(`rule:core-api/tier-placement`). Cross-request state a program wants is explicit and host-owned
(`rule:concurrency/cross-request-state-is-explicit`).
