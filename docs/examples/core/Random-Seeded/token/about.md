Makes a hex text from a seeded generator, such as a fixed ID for a test user or a test order.

You make the generator with `new Core\Random\Seeded(42)`. The number `42` is the seed. Two
generators with the same seed return the same tokens, on every run. By default `token` draws 32
bytes, and the result is a string of 64 characters. The string is always twice as long as the
number of bytes, and it contains only the characters `0` to `9` and `a` to `f`.

Asking for zero bytes throws an error.

**Good to know:** anybody who knows the seed knows every token. For a session ID, a password reset
link or an API key, use `Core\Random::token`.
