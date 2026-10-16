Two terminations never fire the exit queue.

**A `FATAL`.** A resource-limit breach stopped the request *for exceeding its budget*. Running an
unbounded queue of arbitrary hooks after that point is either an unenforceable cap or a reserved
slice sized for work that cannot be sized — the reasoning behind `rule:errors/on-limit`'s single
handler on a single reserved slice. The one observer of a `FATAL` stays `Core\Fatal::onLimit`, and
a program that must see a fatal registers there, because no exit hook ever does. A limit breach *inside* a hook is a `FATAL` like any other: the ladder
takes over and the rest of the queue never runs.

**A cancellation.** `rule:concurrency/cancellation-runs-no-user-code` extends to this queue
unchanged, and for the same reason: every producer of a cancellation — a sibling threw, the deadline
expired, the parent died, the client disconnected at a safepoint — means the request is already
failing or already gone. Process death is the same answer for free.

The firing set is therefore narrower than "every ending". Someone who assumed "no matter what" learns the two
exceptions, and gets `Core\Fatal::onLimit` for the one that matters.
