The response is on the wire before deferred work runs, so **the deferred callable may not touch it**. A write
to `Core\Response` is a compile-time diagnostic where the call is statically visible and a throw
otherwise. `Core\Request`, `Core\Server` and `Core\Session` stay readable — it is still the same
tree, and the request's own values are still there to read.

An uncaught throw inside the deferred callable goes through the escalation ladder to the log, carrying the
scheduling request's trace id. **The request's `onUncaughtThrow` handler does not fire**: it was
request-local and that request's own execution is over. There is no response left for the throw to
affect, so the log is the whole of what it can reach.

`all` and `map` compose inside deferred work with no special case, because a deferred callable runs as an
ordinary task.

**Not on disk.** An uncaught throw in deferred work reaches the floor log and `onUncaughtThrow` does not
fire (`crates/nvs-runtime/src/deferred.rs:216`), and `Core\Response::slotted` throws `LogicError` once
the main script has ended
(`tests/conformance/core/a-response-slotted-in-after-response-work-throws-a-logic-error.nvst`). There is
no compile-time diagnostic for a `Core\Response` write in deferred work, and no run-time check beyond
`slotted`'s.
