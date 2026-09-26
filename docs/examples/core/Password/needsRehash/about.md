Checks whether a stored password hash is older or weaker than a hash made today. This replaces PHP's
`password_needs_rehash`.

It returns `true` when the hash uses another algorithm, or less memory or fewer passes than
`Core\Password::hash` uses now. Every bcrypt hash from PHP returns `true`. It returns `false` for a
hash that is as strong as a new one, or stronger.

You cannot turn an old hash into a new one without the password. So the moment to update it is at
login: `verify` has just returned `true`, and you have the password. If `needsRehash` returns `true`,
you call `hash` and store the new string. Over time, every active user's hash is upgraded.

When the value is not a password hash at all, `needsRehash` throws a `LogicError`.

**The examples below** check a new hash and a weaker one, check a hash from PHP, and upgrade a stored
hash when a user logs in.
