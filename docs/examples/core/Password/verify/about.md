Checks whether a password matches a stored hash.

You give it the password somebody typed and the hash you stored. It returns `true` when they match
and `false` when they do not. It uses the settings saved inside the hash, so a hash made with older
settings still works.

It reads two kinds of hash: the `$argon2id$` strings from `Core\Password::hash`, and bcrypt
hashes, which start with `$2y$`, `$2a$` or `$2b$`.

When the stored value is not a password hash at all, `verify` throws a `LogicError`. A wrong password
returns `false`, but a broken database value is a bug that somebody should see.

**Good to know:** a stored hash that asks for a huge amount of memory or work throws a
`RuntimeError`. It does not run.

**The examples below** check a login, read a bcrypt hash, and write a login function that handles
a broken stored value.
