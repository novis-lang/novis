`Core\Session::setShortLivedSecret()` saves a secret, such as an access token, in the session of the
current request. A later request from the same visitor reads it back with `Core\Session::getSecret()`.

A short-lived secret is deleted when the visitor closes the browser. `Core\Session::clearShortLived()`
also deletes it when the user signs out. This is the right lifetime for a secret that belongs to a
login. A secret that the device keeps after the browser closes belongs in
`Core\Session::setLongLivedSecret()`.

The value is encrypted with the first key in `$keys`, so the session store only sees encrypted bytes.
`Core\Crypto::generateKey()` creates a key. `Core\Session::get()` returns `null` for the secret.

An empty `$keys` list throws a `LogicError`, and so does a first key that is not 32 bytes long. A
program that is not handling a request has no session, and the call throws a `LogicError`.

**The examples below** show how to save a token, that `get()` cannot read it, and a sign-in and
sign-out with an access token.
