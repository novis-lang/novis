Signs an object and returns a JSON Web Token (JWT). The object is a class with
`#[Core\Json\Derive]` or a shape. Its fields become the claims in the token, written as
`Core\Json::encode` writes them. So a claim can be a number, a list or another object, not only
text.

The key is always a key pair. The private half signs, and the pair's kind chooses the algorithm.
You give the public half to the services that check the token. They check it with
`Core\Jwt::verifyIssued`.

You must give a lifetime. `signObject` adds two claims itself: `iat`, the time of signing, and
`exp`, the time the token expires. The object cannot have fields with these two names.

**The examples below** sign an object and read its claims, sign a list with a key name in the
header, and issue an access token that another service checks.
