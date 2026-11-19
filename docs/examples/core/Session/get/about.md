`Core\Session::get()` returns the value saved under one key in the session of the current request.
It reads a value saved with `Core\Session::setShortLived()` or `Core\Session::setLongLived()`.

If the session has no value under the key, `get()` returns `null`. A key set to `null` gives the
same result. A short-lived value is also `null` after the visitor closes the browser.

The return type is `mixed`. Use `as` to convert the value, for example `as ?int`. Write the default
before the conversion, because `null as string` is an empty string.

The first call to any `Core\Session` method opens the session. Reading does not change the session,
so nothing is written to the store.

`get()` throws a `LogicError` when the program is not handling a request. It throws a
`RuntimeError` when the store cannot be opened, and an `IOError` when the store cannot be reached.

**The examples below** show how to read a value, what a missing key returns, and a page view
counter.
