`Core\Session::start()` opens the session of the current request. Call it once, at the top of every
page that uses the session, before any other method of `Core\Session`. Before it, those methods
throw a `RuntimeError`.

The browser sends the session identifier in a cookie, named `nvsid` by default. `start()` reads the
cookie and loads the session data from the store. If there is no cookie, or the store has no
session under the identifier, `start()` opens a new, empty session. The response then sends the new
identifier in the cookie. An identifier that expired and one that somebody made up give the same
result: a new, empty session.

A client that sends the identifier somewhere else, such as in a header, can pass it to `start()`.

`start()` throws a `LogicError` in a program that is not answering a request. It throws an
`IOError` if the store cannot be reached.

**The examples below** show a visit counter, an identifier sent in a header, and a page that only
signed-in users may see.
