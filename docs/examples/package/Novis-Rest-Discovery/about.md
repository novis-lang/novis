`Novis\Rest\Discovery::fetch()` reads the endpoints of an OAuth 2 server from the metadata document
the server publishes. You give it the address of the server, which is called the issuer.

`fetch` first reads the OpenID Connect document at `/.well-known/openid-configuration`. If the server
replies `404`, it reads the OAuth 2 document at `/.well-known/oauth-authorization-server`. The result
has the token endpoint, the authorization endpoint, the address of the public keys and the DPoP
algorithms the server supports.

`fetch` checks the document before you use it. The issuer in the document must be the issuer you
asked for. Every endpoint must be an `https:` URL. If a check fails, `fetch` throws a `RuntimeError`.

`client()` and `publicClient()` return a `Novis\Rest\Client` for the token endpoint. `keys()` reads
the public keys of the server, so you can check the signature of a token with `Core\Jwt`.

**The examples below** read the endpoints, read the OAuth 2 document when there is no OpenID
document, and handle a document that fails a check.
