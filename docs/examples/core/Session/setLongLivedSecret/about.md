`Core\Session::setLongLivedSecret()` saves a secret in the session of the current request. A later
request from the same visitor reads it back with `Core\Session::getSecret()`.

A long-lived secret stays after the visitor closes the browser, for `[session] long_lived`. Signing out
with `Core\Session::clearShortLived()` keeps it. `Core\Session::clearLongLived()` deletes it.

The session cookie alone is enough to read it after the browser closes. Anybody who opens that browser
can read it, without a login. So use it only for a secret of the device. A secret that belongs to a
login, such as an access token, belongs in `Core\Session::setShortLivedSecret()`.

The value is encrypted with the first key in `$keys`, so the session store only sees encrypted bytes.
`Core\Session::get()` returns `null` for the secret.

An empty `$keys` list throws a `LogicError`, and so does a first key that is not 32 bytes long. A
program that is not handling a request has no session, and the call throws a `LogicError`.

**The examples below** show how to save a device token, which clear deletes it, and how to change to
a new key.
