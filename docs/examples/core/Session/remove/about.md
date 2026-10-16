`Core\Session::remove()` deletes one key and its value from the session of the current request.
After it, `Core\Session::get()` returns `null` for that key. The other keys keep their values.

Removing a key that the session does not have is allowed. It does not throw an error and does not
change the session.

Call `Core\Session::start()` first. Before it, and after `Core\Session::destroy()`, `remove()` throws
a `RuntimeError`. The changed session is saved to the store when the request ends.

**Good to know:** to delete every key at once, use `Core\Session::clear()`.

**The examples below** show how to remove one value, what happens when the key is missing, and a
message that a page shows only once.
