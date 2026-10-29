Signs a user in at an OAuth 2 server and gets a token that lets your program act for that user.

`start` returns a URL. Your program sends the user there to sign in. It also returns a random
`state` and a secret `codeVerifier`. Keep both in the user's session. After the sign-in, the server
sends the user back to your `redirectUri` with a `code` and the `state`.

Call `checkState` first. It throws a `RuntimeError` when the `state` is not the one from `start`.
Then `exchange` sends the `code` and the `codeVerifier` to the server and returns a `Token`. Later,
`refresh` uses the token's refresh token to get a new access token.

**In plain words:** the `codeVerifier` is a password that only your program knows. A stolen `code`
does not work without it.

**Good to know:** when the server sends no new refresh token or scope, `refresh` keeps the old ones.
A `code` or `state` with characters outside printable ASCII throws a `RuntimeError`.

**The examples below** build the sign-in URL, check the return and exchange the code, and then
refresh a token.
