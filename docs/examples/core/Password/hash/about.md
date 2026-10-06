Turns a password into a hash that you can store in a database.

You never store the password itself. You store the string `hash` returns, and later you give it to
`Core\Password::verify` to check a password somebody types. The string starts with `$argon2id$` and
contains the settings, a random salt and the hash itself, so `verify` needs nothing else.

`hash` has no algorithm or cost argument. The library chooses safe settings. When those settings
change, `Core\Password::needsRehash` tells you which stored hashes are older.

Each call uses a new random salt. The same password gives a different string every time, so two users
with the same password do not have the same hash.

**Good to know:** one call uses 19 MiB of memory for a short time. This is on purpose: it makes
guessing passwords slow and expensive.

**The examples below** hash a password, show that two hashes of one password differ, and store a new
user's password at sign-up.
