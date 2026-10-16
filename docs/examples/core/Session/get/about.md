`Core\Session::get()` returns the value saved under one key in the session of the current request.
You save a value with `Core\Session::set()`, and a later request reads it back with `get()`.

If the session has no value under the key, `get()` returns `null`. A key that was set to `null`
gives the same result. Use `??` to give a default.

The return type is `mixed`. Use `as` to convert the value to the type you need, for example
`as ?int`. Write the default before the conversion, because `null as string` is an empty string.

Call `Core\Session::start()` first. Before it, and after `Core\Session::destroy()`, `get()` throws a
`RuntimeError`.

Reading does not change the session. A request that only reads its session does not write it back
to the store.

**The examples below** show how to read a value, what a missing key returns, and a page view
counter.
