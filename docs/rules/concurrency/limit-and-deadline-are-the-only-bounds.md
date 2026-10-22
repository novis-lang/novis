`all` and `map` take one trailing options shape with the same two fields.

**`limit: uint`** is the greatest number of the call's own children running at once. Exceeding it
**schedules** — it never throws. It is a concurrency shaper, deliberately unlike
`rule:concurrency/a-full-deferred-executor-throws`'s host bound, which does throw. Both members
default to unbounded.

**`deadline: Duration`** bounds the whole call, not each child, and is written as a duration
like `30s`. It is a timer on the core's reactor, so it fires when the core is handed back: a child
waits, ends, or uses up its time slice. **A child that never waits is stopped by the deadline too**,
one slice after it at most, because the slice clock (`crates/nvs-host/src/slice.rs`) asks a turn of
10 ms or more to yield and the yield polls the reactor before the child runs again. A group gets no
stop of its own: the slice already hands the core to the reactor, so a deadline needs no word the
watchdog has to find. The request's `[limits] cpu_time` still bounds the request as a whole, raised on
the request tree's safepoint word that every child polls. There is no default deadline: a call naming
none is bounded by its request's `wall_time` and `cpu_time`. `wall_time` is uncapped unless the
deployment writes it, and `cpu_time` is 30 seconds for server work where it is not written
(`rule:config/no-configuration-file-is-a-complete-configuration`).

These two are the whole vocabulary, and an optional argument is an options field, never a wrapper
function (`rule:core-api/options-bag`).

**Declined:**
- **A `timeout` member.** A timeout over a group is `deadline`, and a timeout on one I/O call is
  that call's own `timeout` option (`rule:core-classes/db-one-api`).
- **`race`.** Over a heterogeneous shape its answer is a union the caller must discriminate, which
  is `mixed` and a cast in practice. The case actually wanted, hedging one request across two
  replicas, is homogeneous: `Core\Task::first(array<T>, callable, {deadline?})` is the name kept for
  it, so it cannot arrive as a second `race`, and it lands additively when a program needs it.
