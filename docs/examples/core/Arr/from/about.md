Builds an array out of anything a `foreach` can walk.

The source can be an array, a generator, or an object that gives values one at a time.
`Core\Arr::from` reads the source once, from the first value to the last, and returns a list. The
result is numbered from `0`, so the keys of a source array are not kept. A generator runs only as
far as it is read.

The `limit` option stops the reading after that many values. This is what you use for a sequence
that never ends on its own. Without a limit, such a sequence is read until the program reaches its
memory limit and stops with an error. A `limit` of `0` reads nothing at all. It replaces PHP's
`iterator_to_array`.

**The examples below** show a generator collected into an array, an array with keys turned into a
plain list, and the first five values of a sequence that never ends.
