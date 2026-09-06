`all` and `map` take one trailing options shape with the same two fields.

**`limit: uint`** is the greatest number of the call's own children running at once. Exceeding it
**schedules** — it never throws. It is a concurrency shaper, deliberately unlike
`rule:concurrency/a-full-deferred-executor-throws`'s host bound, which does throw. Both members
default to unbounded.

**`deadline: Duration`** bounds the whole call, not each child, and is written as a duration
literal. There is no default: a call naming no deadline is bounded by its request tree's own
`wall_time`, which is finite, and that is the only reason omitting it is safe.

These two are the whole vocabulary. There is no `timeout` member — a timeout over a group is this
option, and a timeout on one I/O call is that call's own option — and there is no `race`: over a
heterogeneous shape its answer would be a union the caller must discriminate, which is `mixed` and a
cast in practice. The homogeneous case that is actually wanted, hedging one request across two
replicas, has a name reserved for it so it cannot arrive twice.
