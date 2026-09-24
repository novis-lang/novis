Signs a list of claims and returns a JSON Web Token (JWT). A claim is a name and a text value, such
as a user id or a role. The token is a short `string` you can send in a cookie, a header or a URL.
Anybody can read the claims in it, but nobody can change them without the key.

The key chooses the algorithm. A secret key of 32 bytes or more signs with `HS256`, and your own
program checks the token with `Core\Jwt::verify`. A key pair signs with the algorithm of its kind,
and other services check the token with the public half.

You must give a lifetime. `sign` adds two claims itself: `iat`, the time of signing, and `exp`,
the time the token expires. You cannot write these two claims yourself.

**The examples below** sign and read back a token, sign with a key pair and a key name, and check
a login cookie on each request.
