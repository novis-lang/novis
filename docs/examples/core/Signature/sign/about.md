`Core\Signature::sign` signs a set of named values with a secret key and returns a token. Later,
`Core\Signature::verify` checks the token with the same key and returns the values. If anybody
changed one character of the token, `verify` throws an error. This is how you make a link or a
reset code that a user can carry but cannot edit.

You give `sign` two settings, and you must write both. `keys` is a list of keys, newest first, and
the newest key signs. `until` is the time the token stops being valid, or `null` for a token that
never expires. The time is inside the signed data, so nobody can change it.

The token is not encrypted. Anybody who has it can read the values, so never sign a password or
another secret. The same values and settings always give the same token.

**The examples below** sign and check a token, show that the order of the names does not matter,
and build a password reset link that expires after one hour.
