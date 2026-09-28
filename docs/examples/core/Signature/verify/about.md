`Core\Signature::verify` checks a token that `Core\Signature::sign` made, and returns the values
that were signed. It tries each key in the list you give it. If no key made the token, or somebody
changed the token, it throws a `RuntimeError`. If the token has expired, it also throws a
`RuntimeError`, with a message that gives the time it expired.

Every value it returns is a `tainted string`. Tainted means Novis treats the value as input from
outside your program. A valid signature shows that your program made the token. It does not show
that the values are safe to put in an HTML page or a database query. The values come back sorted by
name.

You can add a new key at the start of the list and keep the old keys after it. Tokens made with an
old key still work until you remove that key.

**The examples below** read the values from a token, catch a changed and an expired token, and
change to a new key without breaking the tokens users already have.
