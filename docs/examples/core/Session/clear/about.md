`Core\Session::clear()` removes every value from the session of the current request. After it,
`Core\Session::get()` returns `null` for every key.

The session itself stays open. The client keeps the same session cookie, and you can call
`Core\Session::set()` again at once, without a second `Core\Session::start()`. The empty session is
saved to the store when the request ends.

Call `Core\Session::start()` first. Before it, `clear()` throws a `RuntimeError`. Clearing a session
that is already empty is allowed and changes nothing.

**Good to know:** `clear()` keeps the session identifier. To end a session completely, for example
when a user signs out, use `Core\Session::destroy()`.

**The examples below** show an emptied session, a session that is used again after `clear()`, and a
"Start over" button for an order form.
