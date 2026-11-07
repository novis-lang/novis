An upgraded connection is a **root isolate**, not a suspended request: its own arena, its own globals
and statics, its own `[limits]` budget, its own capability grants, and its own entry in the timeline
and the metric series. It shares immutable compiled code with everything else in the process and
nothing more (`rule:security/isolate-shares-nothing`).

**The request that upgraded it ends normally.** A connection does not hold that request's arena and
cannot see its session, cookies or headers unless a value was explicitly passed in. That is the
security property the model is worth paying for: authentication happens in an ordinary HTTP request
with every server default applied, and what reaches the long-lived isolate is whatever the
application chose to hand it — an account id, a tenant id — never the credential that proved it.

A connection isolate and deferred `Core\Task::afterResponse` work are the only two things in this
language that outlive a response, and they are opposites at every point that matters: deferred work
belongs to the request tree, sees the request's heap and spends the request's remaining budget, while
a connection is a root, sees only what its arguments copied in, and ends when the peer closes, a
timeout fires or a budget is exceeded. A fatal error inside one tears down the connection and nothing
wider (`rule:errors/escalation-ladder`), and the peer is told with a defined close code.

**`wall_time` bounds the request that upgrades, and never the connection.** The connection starts a
limit tree of its own, so `memory` and `cpu_time` apply to it from a fresh count, and its tree states
no wall deadline. What ends it in time is `rule:concurrency/connection-bounds-are-finite`'s lifetime,
which every deployment has and no `upgrade` can remove. A connection that kept the request's deadline
closed at it: every event stream under a `wall_time` ended and reconnected, and an operator who
followed the security checklist broke every live page. An application cannot exempt one door either,
because `limits:` on an upgrade only narrows.

- **Memory:** each open connection holds an arena and a task stack for as long as the peer keeps it
  open, idle or not. Memory scales with concurrent connections and not with the request rate, so a
  connection-heavy deployment is sized for its connections, and ten thousand idle connections cost ten
  thousand arenas.

**Declined:**
- **A connection as a suspended request.** A request's budget, teardown and observability all assume
  it ends, and a request held open for hours would keep its session and cookies for the life of the
  socket, which is what ending the upgrading request prevents.
- **The request's `wall_time` as a connection's bound too.** A request's time budget is sized for a
  request, in seconds, and a connection lives for minutes or hours, so one number cannot be right for
  both; the lifetime is the bound sized for a connection.
- **A dedicated connection worker pool outside the request path.** It is a second execution model,
  with its own scheduling, isolation and security review, bought for connection counts the audience
  does not have.
