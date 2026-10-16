Checks a JSON Web Token (JWT) that another service signed, such as a login provider, and returns
its claims as a type you choose. You give the token, the service's public key or its published key
set, the issuer you expect and the audience you expect.

`verifyIssued` checks the token in a fixed order. It checks the signature first. Only a token with a
valid signature gets the next checks: the expiry and start time, then the `iss` claim (who issued the
token) and the `aud` claim (who the token is for). If one check fails, it throws a `RuntimeError`.
All signature failures give the same message, so an attacker learns nothing from it. It never
returns an empty or false value.

The claims come back as the type between `<` and `>`, in the same way as `Core\Json::decodeAs`.
Its text fields must be `tainted`. A valid signature proves who wrote a claim, not that it is safe
to use in HTML or SQL.

**The examples below** read the claims of a token, show a token for another application being
refused, and find the user of a request with the key set a login service publishes.
