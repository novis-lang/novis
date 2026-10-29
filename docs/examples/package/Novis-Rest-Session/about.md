`Novis\Rest\Session` saves the OAuth 2 tokens of a signed-in user in the user's session. A later
request from the same user reads them back.

`save` saves a `Novis\Rest\Token`. The access token and the refresh token are encrypted with
`$keys`, so the session store only sees encrypted bytes. `load` returns the saved token, or `null`
when the session has none. This token may be expired.

`current` returns a token that is not expired. When the saved token is expired, `current` sends the
refresh token to the server and saves the new token. It returns `null` when the user is not signed
in, or when the expired token has no refresh token.

`forget` deletes the tokens from the session, for example when the user signs out. Other values in
the session stay.

`load` checks every value it reads from the session store. A value with the wrong type, or with
characters a token may not have, gives `null`.

Call `Core\Session::start()` first. `$keys` is a list of encryption keys, newest first.
`Core\Crypto::generateKey()` creates a key.
