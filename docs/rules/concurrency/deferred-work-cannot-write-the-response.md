The response is on the wire before deferred work runs, so **the deferred callable may not touch it**. Every
`Core` member that sets a status, a header, a cookie or a body, `Core\Response::slotted` and
`Core\Session::regenerate` is refused in it: a compile error, `E0857`, where the call is written inside
the anonymous function passed to `Core\Task::afterResponse` or one nested in it, and a `LogicError` at run
time otherwise — a callable held in a variable, a method it calls, and every task the work starts.
`Core\Request`, `Core\Server` and `Core\Session` stay readable — it is still the same tree, and the
request's own values are still there to read. `echo` in deferred work is the work's own output and is not
a response write.

The compile-time reach is lexical, the same as `Core\Html::later`'s closed head
(`rule:core-classes/html-later`), because which bodies a callable reaches is a whole-program question and
the run-time throw already covers the rest. The throw is read off one byte on the context that the drain
sets and every child task copies (`crates/nvs-runtime/src/deferred.rs`), so a request that defers nothing
pays one branch per response member.

An uncaught throw inside the deferred callable goes through the escalation ladder to the log, carrying the
scheduling request's trace id. **The request's `onUncaughtThrow` handler does not fire**: it was
request-local and that request's own execution is over. There is no response left for the throw to
affect, so the log is the whole of what it can reach.

`all` and `map` compose inside deferred work with no special case, because a deferred callable runs as an
ordinary task.
