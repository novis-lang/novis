`Core\Session::setSecret()` saves a secret in the session of the current request. A secret is a
value that nobody else may read, for example an access token or a refresh token of the user. A
later request from the same visitor reads it back with `Core\Session::getSecret()`.

`setSecret()` encrypts the value before it is saved. The session store only sees the encrypted
bytes. `$keys` is a list of encryption keys, newest first, and the first key encrypts the value.
`Core\Crypto::generateKey()` creates a key.

`Core\Session::get()` returns `null` for a key that holds a secret. Only `getSecret()` with the
right keys can read it. The secret stays readable after `Core\Session::regenerate()`.

Call `Core\Session::start()` first. Before it, `setSecret()` throws a `RuntimeError`. An empty
`$keys` list throws a `LogicError`.

**The examples below** show how to save a token, that `get()` cannot read it, and a refresh token
that is saved when a user signs in.
