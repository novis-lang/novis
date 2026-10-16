`Core\Session::set()` saves a value under a key in the session of the current request. A later
request from the same visitor reads it back with `Core\Session::get()`.

If the key already has a value, `set()` replaces it. The value can be a string, a number, a bool,
`null`, an array or an object. A callable cannot be saved, and `set()` throws a `LogicError` for it.

Call `Core\Session::start()` first. Before it, and after `Core\Session::destroy()`, `set()` throws a
`RuntimeError`.

The session is saved to the store once, when the request ends. Many calls to `set()` in one request
still write to the store only one time.

**The examples below** show how to save a value, how a new value replaces the old one, and a
shopping cart that is kept between pages.
