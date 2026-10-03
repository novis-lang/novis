`all` and `map` take one trailing options shape with the same two fields.

**`limit: uint`** is the greatest number of the call's own children running at once. Exceeding it
**schedules** — it never throws. It is a concurrency shaper, deliberately unlike
`rule:concurrency/a-full-deferred-executor-throws`'s host bound, which does throw. Both members
default to unbounded.

**`deadline: Duration`** bounds the whole call, not each child, and is written as a duration
literal. It is a timer on the core's reactor, so it fires when the core is handed back — a child
waits or ends. **A child that never waits is bounded the way its request is**: by the request's
`[limits] cpu_time`, which the watchdog raises on the request tree's safepoint word and every child
polls. A group gets no stop of its own, because it would bound a child more tightly than its request
and cost every group a word the watchdog has to find. There is no default deadline: a call naming
none is bounded by its request's `wall_time` and `cpu_time`, and both are uncapped unless the
deployment writes them.

These two are the whole vocabulary. There is no `timeout` member — a timeout over a group is this
option, and a timeout on one I/O call is that call's own option — and there is no `race`: over a
heterogeneous shape its answer would be a union the caller must discriminate, which is `mixed` and a
cast in practice. The homogeneous case that is actually wanted, hedging one request across two
replicas, has a name reserved for it so it cannot arrive twice.
