`Core\Session::getSecret()` reads a secret that `Core\Session::setSecret()` saved in the session.
A secret is a value that nobody else may read, for example an access token of the user.

The secret is saved encrypted. `getSecret()` decrypts it with a key from `$keys`. Every key in the
list is tried, so you can add a new key at the front and still read secrets that an older key
encrypted.

The result is `null` when nothing is saved under the key, and also when no key in `$keys` can
decrypt the secret. The result is the same `null` in every case, so a program cannot learn anything
about a key it does not have. Reading does not change the session.

Call `Core\Session::start()` first. Before it, `getSecret()` throws a `RuntimeError`. An empty
`$keys` list throws a `LogicError`.

**The examples below** show how to read a secret, how to change keys without losing secrets, and a
page that needs the user's access token.
