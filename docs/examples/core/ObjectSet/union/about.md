Builds a new `Core\ObjectSet` with every value that is in this set, in `$other`, or in both. A value
that is in both sets is in the result only once. This replaces a loop that copies two
`SplObjectStorage` objects into a third one.

A value is matched by identity. Two objects with equal fields are two different values, so the
result has both of them.

The result is a new set. `union` does not change this set or `$other`. The values of this set come
first, in their order. Then the values of `$other` that are not in this set follow, in their order.

The examples show two sets combined into one, a value that is in both sets, and a common use: one
list of people to send a message to, where nobody gets the message twice.
