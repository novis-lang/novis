`Core\Session::regenerate()` gives the session of the current request a new identifier. The
identifier is the value in the session cookie that the browser sends back with every request.

Call `regenerate()` when a user signs in. Somebody else may have known the old identifier, for
example because they gave the user a link that contains it. After `regenerate()`, the old
identifier no longer finds the session, so it is useless to them.

The data in the session does not change. Values saved with `Core\Session::set()` and secrets saved
with `Core\Session::setSecret()` can still be read. The store is updated at once, not at the end of
the request, and the response sends the new identifier in the session cookie.

Call `Core\Session::start()` first. Before it, `regenerate()` throws a `RuntimeError`. If the store
cannot be reached, it throws an `IOError`.

**The examples below** show a new identifier after sign-in, that values and secrets are kept, and
a sign-in page.
