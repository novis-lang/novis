Puts every value of an array in a random order, for example to shuffle a deck of cards or to show
answers in a different order each time.

`Core\Random::shuffle` returns a new list with all the values of the array. Every possible order
has the same chance. The keys of the new list start again at `0`, so the old keys are not kept. The
array you pass in does not change. An empty array gives an empty list. The order comes from a
cryptographic random generator, so nobody can predict it. This replaces PHP's `shuffle`, which
changes the array itself.

**Good to know:** when you need only some of the values, use `Core\Random::sample`.
