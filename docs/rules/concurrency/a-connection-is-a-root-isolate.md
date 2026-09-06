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
