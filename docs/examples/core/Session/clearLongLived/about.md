`Core\Session::clearLongLived()` deletes every long-lived value from the session of the current
request. The short-lived values are kept.

A long-lived value is the kind a preference uses, such as a cookie choice or a cart. It is kept after
the browser closes. A short-lived value is the kind a sign-in uses. The browser deletes it when it
closes. Call `clearLongLived()` when a visitor withdraws a consent, and the user stays signed in. To
sign a user out, use `Core\Session::clearShortLived()`.

A key has the lifetime of its last write. The session also gets a new identifier. The session
cookie no longer has a `Max-Age` (the time the browser keeps a cookie), so the browser deletes it
when it closes. The session stays open, so you can save new values at once.

It throws a `LogicError` outside a request, or inside `Core\Html::later` or
`Core\Task::runAfterResponse`. It throws a `RuntimeError` when the session store (the place where
the server keeps session data) cannot be opened. It throws an `IOError` when the store cannot be
reached.

**The examples below** delete preferences and keep the user signed in. They also show the lifetime
of the last write, and a button that withdraws a cookie consent.
