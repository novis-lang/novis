Only the request's own task may register deferred work. A `Core\Task` child and a deferred callable
are each born sealed, and a registration made on one is refused with a `RuntimeError` at the call
site — while there is still a request to decide what to do about it — rather than accepted and then
dropped when that child ends.

The queue belongs to one request, so it is the only thing that can drain it. A child that wants to
defer hands the work back to the request that started it; that is what the refusal costs, and what
it buys is that no registration is ever silently lost.

**Deferred work may not defer more.** That is the same rule read once more rather than a second one:
a deferred callable runs as a child, on a sealed context, like every other child. A queue able to
extend itself is a tree that never leaves flight, and the whole argument for
`rule:concurrency/after-response-outlives-the-connection` is that the tree outlives the connection
only a little.
