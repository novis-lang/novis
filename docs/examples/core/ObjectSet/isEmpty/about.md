Checks if a `Core\ObjectSet` has no values. `isEmpty` returns `true` for an empty set, and `false`
when the set has at least one value. This replaces `count($storage) === 0` on an
`SplObjectStorage`.

A new set is empty. A set is empty again after `clear`, or after `remove` takes out its last value.
`isEmpty` gives the same answer as checking that `count` is `0`, and it is shorter to read.

`isEmpty` does not change the set.

The examples show a set before and after it gets values, a set that is empty again after `remove`,
and a common use: a message that is shown only when there is something to report.
