Chooses several different values from an array at random, for example three winners of a prize draw
or five questions for a quiz.

`Core\Random::sample` takes the array and the number of values you want. It returns a new list with
that many values. Each entry of the array is chosen at most once. Every possible choice has the same chance. The values
come back in a random order, and their keys start again at `0`. The array itself does not change. If
you ask for more values than the array has, `Core\Random::sample` throws a `RuntimeError`. The choice
comes from a cryptographic random generator, so nobody can predict it.

**Good to know:** to put every value in a random order, use `Core\Random::shuffle`.
