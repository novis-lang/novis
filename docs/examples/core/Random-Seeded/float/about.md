Returns a `float` from a seeded generator. The result is at least 0 and less than 1.

You make the generator with `new Core\Random\Seeded(42)`. The number `42` is the seed. Two
generators with the same seed return the same floats in the same order, on every run. This makes a
simulation repeatable: you can run it again and get the same result. The result can be `0.0`, but it
is never `1.0`. Multiply it by 10, and the result is at least 0 and less than 10.

**Good to know:** anybody who knows the seed knows every value. When nobody may guess the value, use
`Core\Random::float`.
