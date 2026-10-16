Chooses several different values from an array with a seeded generator, so the same values come
back on every run.

You make the generator with `new Core\Random\Seeded(42)`. The number `42` is the seed. Two
generators with the same seed choose the same values in the same order. `sample` takes the array
and the number of values you want, and returns a new list with that many values. Each entry of the
array is chosen at most once. The keys of the new list start again at `0`, and the array does not
change.

If you ask for more values than the array has, `sample` throws an error.

**Good to know:** anybody who knows the seed knows every choice. For a prize draw, use
`Core\Random::sample`.
