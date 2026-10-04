A cancelled task is torn down by the runtime where it parked. **No `catch` clause runs, no
`finally`-shaped cleanup runs, and no registered handler runs.** Native teardown still runs, because
none of it is script: the arena is dropped, refcounts are released, an open transaction is rolled
back by its own drop, an open file is closed.

Cancellation is therefore **not a `Throwable`** and cannot be caught, exactly as a resource-limit
report is not one. There are three ways to be cancelled — a sibling threw, a deadline expired, or
the parent died, which for a served request includes a client disconnecting when its method is
listed in `cancel_on_disconnect` or its `disconnect_grace` runs out
(`rule:http-server/a-request-outlives-a-client-that-goes-away`) — and all three mean the request is
already failing. Running arbitrary user cleanup there means running unbudgeted code
inside a failure, which is how a timeout becomes a hang, and a `catch (Throwable)` that swallowed a
cancellation would do exactly that.

The consequence to learn: **work that must happen does not go in a cancellable task's cleanup.** It
goes inside the transaction that made it necessary, in
`rule:concurrency/after-response-outlives-the-connection`'s deferred work, or in a durable queue.
