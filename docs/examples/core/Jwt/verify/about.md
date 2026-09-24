Checks a JSON Web Token (JWT) that your own program signed with `Core\Jwt::sign`, and returns its
claims. You give the token and the same secret key that signed it.

`verify` checks two things. First, the signature: the token must be signed with this key, and
nobody may have changed it. Second, the expiry: the time in the `exp` claim must not have passed.
If one check fails, `verify` throws a `RuntimeError`. It never returns an empty or false value, so
you cannot forget to check the result.

The claims come back as an array of text, `iat` and `exp` included. Each value is `tainted`,
because it came from outside your program. A valid signature proves who wrote a claim, not that it
is safe to use in HTML or SQL.

**The examples below** read the claims of a token, show that a changed token throws an error, and
find the user of an API request.
