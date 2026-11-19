`Core\Session::openWithIdentifier($id)` opens the session of the current request under an
identifier that your program read itself. A mobile app, for example, may send it in a header. Most
programs do not need this method: the first call of any other `Core\Session` method opens the
session from the session cookie.

`openWithIdentifier()` reads no cookie and sends no cookie. A short-lived value in this session
needs only the identifier, because there is no browser that could close.

If the store has no session under `$id`, the result is a new, empty session. This happens when the
identifier expired, and when somebody made it up.

It must be the first `Core\Session` call in the request. Otherwise it throws a `LogicError` that
names the method that opened the session. It also throws a `LogicError` outside a request. It throws
a `RuntimeError` if the store cannot be opened, and an `IOError` if it cannot be reached.

**The examples below** show an identifier sent in a header, a call that comes too late, and a page
for both an app and a browser.
