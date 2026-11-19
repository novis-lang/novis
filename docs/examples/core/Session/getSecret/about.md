`Core\Session::getSecret()` reads a secret from the session, for example an access token of the
user. It reads a secret that `Core\Session::setShortLivedSecret()` or
`Core\Session::setLongLivedSecret()` saved. A short-lived secret ends when the visitor closes the
browser. A long-lived secret stays after that.

The secret is saved encrypted, and `getSecret()` tries every key in `$keys`. So you can add a new
key at the front and still read secrets that an older key encrypted.

The result is `null` when nothing is saved under the key, and when no key can decrypt the secret.
Both cases give the same `null`, so a program learns nothing about a key it does not have.
`Core\Session::get()` returns `null` for a secret.

Outside a request, `getSecret()` throws a `LogicError`. An empty `$keys` list, or a key that is not
32 bytes long, also throws a `LogicError`. A store that cannot be opened throws a `RuntimeError`,
and a store that cannot be reached throws an `IOError`.

**The examples below** show how to read a secret of each lifetime, how to change keys without
losing secrets, and a page that needs the user's access token.
