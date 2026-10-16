Puts every value of an array in a random order with a seeded generator, so the same order comes
back on every run.

You make the generator with `new Core\Random\Seeded(42)`. The number `42` is the seed. Two
generators with the same seed give the same order. This is useful when a test runs its steps in a
random order: if one order fails, the same seed repeats it. `shuffle` returns a new list with all
the values of the array. The keys of the new list start again at `0`, and the array you pass in
does not change. An empty array gives an empty list.

**Good to know:** anybody who knows the seed knows every order. For a card game with real players,
use `Core\Random::shuffle`.
