Returns the number of bytes you ask for from a seeded generator.

You make the generator with `new Core\Random\Seeded(42)`. The number `42` is the seed. Two
generators with the same seed return the same bytes, on every run. This is useful for test data,
such as the content of a file that a test uploads. The result is a `bytes` value of exactly the
length you asked for. Convert it with `Core\Encoding::toHex` to show it as text.

Asking for zero bytes throws an error.

**Good to know:** anybody who knows the seed knows every byte, so never use these bytes as a key,
a salt or a nonce. For those, use `Core\Random::bytes`.
