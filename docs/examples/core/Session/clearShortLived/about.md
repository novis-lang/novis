`Core\Session::clearShortLived()` deletes every short-lived value from the session of the current
request. The long-lived values are kept.

Call it when a user signs out. A short-lived value is the kind a sign-in uses. The browser deletes it
when it closes. A long-lived value, such as a cookie choice or a cart, is kept after the browser
closes. To delete only those, use `Core\Session::clearLongLived()`.

The session also gets a new identifier, and the response sends it in the session cookie. A copy of the
old cookie no longer finds the session. The session stays open, so you can save new values at once.

It throws a `LogicError` outside a request, or inside `Core\Html::later` or
`Core\Task::runAfterResponse`. It throws a `RuntimeError` when the session store (the place where
the server keeps session data) cannot be opened. It throws an `IOError` when the store cannot be
reached. The old identifier may then still work, so do not treat the user as signed out.

**The examples below** show a sign-out that keeps a preference, a session that is used again at once,
and a sign-out page.
