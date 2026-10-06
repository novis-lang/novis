Draws the number of random bytes you ask for, for a key, a salt or a nonce.

The bytes come from a cryptographic random generator, so nobody can guess the next value from the
values before it. The result is a `bytes` value of exactly the length you asked for. It is raw data
and not text, so you convert it with `Core\Encoding::toHex` or `Core\Encoding::toBase64` before you
store it or show it.

Asking for zero bytes throws an error. An empty key is equal to every other empty key, so it
protects nothing.

**Good to know:** when you want random text for a link or a session, `Core\Random::token` returns
it as hex in one call.
