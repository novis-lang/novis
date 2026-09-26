Chooses one value from an array with a seeded generator, so the same choices come back on every run.

You make the generator with `new Core\Random\Seeded(42)`. The number `42` is the seed. Two
generators with the same seed choose the same values in the same order. This is useful for test
data, such as the product in each test order. Every value in the array has the same chance.
`pick` returns the value itself, not its key, and the array does not change. If the array is
empty, the result is `null`.

**Good to know:** anybody who knows the seed knows every choice. For a choice nobody may predict,
use `Core\Random::pick`.
