Chooses one value from an array at random, for example a tip of the day or a server to connect to.

Every value in the array has the same chance to be chosen. `Core\Random::pick` returns the value
itself, not its key, and the array does not change. If the array is empty, the result is `null`.
Because of that, the result type is nullable, and you check for `null` before you use it. The
choice comes from a cryptographic random generator, so nobody can predict it. This replaces PHP's
`$a[array_rand($a)]`.

**Good to know:** when you need more than one value and no value twice, use
`Core\Random::sample`.
