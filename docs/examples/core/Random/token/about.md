Makes a random text for a session ID, a password reset link or an API key.

`Core\Random::token` draws random bytes from a cryptographic random generator and writes them as
hex. Nobody can guess the next token from the tokens before it. By default it draws 32 bytes, and
the result is a string of 64 characters. You can ask for another number of bytes. The string is
always twice as long as that number, and it contains only the characters
`0` to `9` and `a` to `f`.

Asking for zero bytes throws an error. An empty token is equal to every other empty token, so it
protects nothing.

**Good to know:** when you need the raw bytes for a key, use `Core\Random::bytes`.
