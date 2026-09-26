Returns a whole number between two numbers from a seeded generator. Both numbers can be returned.

You make the generator with `new Core\Random\Seeded(42)`. The number `42` is the seed. Two
generators with the same seed return the same numbers in the same order, on every run and on every
computer. This makes a test or a simulation repeatable. `$generator->int(1, 6)` rolls a die, and
each value from 1 to 6 comes up equally often.

The first number must not be larger than the second. If it is, the method throws an error.

**Good to know:** anybody who knows the seed knows every number. For a login code or a PIN, use
`Core\Random::int`.
