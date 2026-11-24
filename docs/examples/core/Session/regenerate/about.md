`Core\Session::regenerate()` gives the session of the current request a new identifier. The
identifier is the value in the session cookie. Call it when a user signs in. Somebody else may have
known the old identifier. After `regenerate()`, the old identifier no longer finds the session.

Every value is kept: short-lived values, long-lived values and secrets of both lifetimes. A value
that another request saved in the meantime is kept too. The response sends the new identifier in
the session cookie. A session with nothing saved has no identifier, so `regenerate()` does nothing.

To sign a user out, use `Core\Session::clearShortLived()`. It also gives a new identifier, but it
deletes the short-lived values.

Outside a request, `regenerate()` throws a `LogicError`. Inside `Core\Html::later` or
`Core\Task::runAfterResponse` the call does not compile or throws a `LogicError`. The session store
is the place where the server keeps session data. A store that cannot be opened throws a
`RuntimeError`, and a store that cannot be reached throws an `IOError`.

**The examples below** show a new identifier after sign-in, that values and secrets are kept, and
a sign-in page.
