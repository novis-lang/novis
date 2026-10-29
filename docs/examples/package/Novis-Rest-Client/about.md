An OAuth 2 client that gets access tokens from a token server and sends API requests with them.

You create a `Client` with the URL of the token endpoint and your client id. A server program that
can keep a secret uses `Client::confidential`. A command line program uses `Client::withoutSecret`.
`requestToken` sends a form to the token endpoint and returns a `Token`. `send` and `get` send a
request with the token in the `Authorization` header.

`withDPoP` returns a client that signs every request with its own key. The server can then bind the
token to that key.

**In plain words:** a normal token works like a key card that anybody can use. A DPoP token works
only together with the key that this client has.

**Good to know:** an error reply from the token server throws a `RuntimeError` with the server's
error code. A token with characters a header may not have also throws a `RuntimeError`.

**The examples below** show how to request a token, how to call an API with it, and how to bind the
token to a key.
