`Novis\Rest\DPoP` signs a DPoP proof for each request that your program sends with an access token.

**In plain words:** a DPoP proof is a short signed note. It says "this request comes from the
program that has this key". The server can then link a token to that key. A person who steals the
token cannot use it, because they do not have the key.

`DPoP::generate()` creates a new key pair. Keep the `DPoP` object as long as the tokens that are
linked to its key. The method `withDPoP()` of `Novis\Rest\Client` returns a client that adds a proof
to every request. You only call `proof()` yourself when you send a request without that client.

Each proof contains the method, the URL without its query, and a hash of the access token. A proof
is valid for one minute.

**Good to know:** a server may send a `DPoP-Nonce` header. The client then sends the request once
more, with the nonce in the new proof. A nonce with characters that a header may not have throws a
`RuntimeError`.
