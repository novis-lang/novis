Returns how many values a `Core\ObjectSet` has, as a `uint`. An empty set has a count of `0`.
This replaces `count($storage)` on an `SplObjectStorage`.

Each value is in a set once, so the count is the number of distinct values. Adding a value that is
already in the set does not change the count. Two objects with equal fields are two values, and
they count as two.

`count` does not change the set. To check only if the set is empty, `isEmpty` is shorter.

The examples show the count after adding and removing, the number of distinct values in a list,
and a common use: a limit on how many different guests can join an event.
