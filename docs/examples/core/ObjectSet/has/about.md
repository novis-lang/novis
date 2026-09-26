Checks if a value is in a `Core\ObjectSet`. `has` returns `true` when the set has the value, and
`false` when it does not. This replaces `$storage->contains($object)` on an `SplObjectStorage`.

A value is matched by identity. `has` returns `true` only for the same object that was added. An
object with equal fields is a different object, so the result is `false`. For a scalar such as an
`int` or a `string`, an equal value is the same value.

`has` does not change the set. It is fast, even for a very large set, because it does not look at
every value.

The examples show `has` before and after `add` and `remove`, an equal object that is not the same
object, and a common use: skipping a job that was already done.
