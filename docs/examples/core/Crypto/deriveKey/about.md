Turns a password into a key, by putting it through a slow calculation many times over.

A password is short and people reuse it, so it is never used as a key directly. This member stretches
it instead. You give it three things: the password, a salt, and how many rounds to run. The salt is
at least 16 bytes, drawn once per password with `Core\Random::bytes(16)`. It is not a secret, and it
is stored next to the key it produced. The number of rounds is between 100,000 and 2,000,000, and
you have to name it, because a key you want to read back later must be produced with the same number
you chose the first time.

The same password, salt and number of rounds always give the same key.

**Good to know:** this is the member for a value a person typed. For material that is already
random, such as a shared secret, use `Core\Crypto::expandKey`.
