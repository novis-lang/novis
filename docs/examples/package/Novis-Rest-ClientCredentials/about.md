Gets an access token for a server program itself, with its own client id and secret and no user.

A server program uses this grant to call another service. `ClientCredentials::token` returns a token
for a scope (the list of permissions the token gives). It keeps the token in the memory cache of the
process until 30 seconds before it expires. So most calls send no request to the token server.

The cache stores the token encrypted with the keys you pass. `Core\Crypto::generateKey()` creates a
key. `ClientCredentials::request` always asks the server for a new token and does not use the cache.

When the server replies with an error, both methods throw a `RuntimeError`. The message has the
status and the error code from the reply. A token, a token type or a scope with characters that are
not allowed also throws a `RuntimeError`.

**The examples below** show a token for a scope, a second call that uses the cached token, and an
error reply from the server.
