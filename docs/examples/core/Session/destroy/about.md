`Core\Session::destroy()` ends the session of the current request. It deletes the session data from
the store at once, not at the end of the request. Call it when a user signs out.

After `destroy()`, the request has no session. `Core\Session::get()`, `Core\Session::set()` and the
other methods throw a `RuntimeError`, as they do before `Core\Session::start()`. If you call
`start()` again, it opens a new, empty session with a new identifier.

The client may still send the old session cookie. That identifier no longer names any data, so the
next `start()` ignores it and issues a new one.

If the store cannot be reached, `destroy()` throws an `IOError`. The data stays in the store and the
session stays open, so you can try again.

**Good to know:** to remove the values but keep the session, use `Core\Session::clear()`.

**The examples below** show a sign-out, the error after it, and a sign-out page.
