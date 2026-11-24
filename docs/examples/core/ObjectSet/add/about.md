Puts a value into a `Core\ObjectSet`. If the set already has that value, `add` does nothing, so
each value is in the set once. `add` returns nothing. Use `has` to check if a value is in the set.
This replaces `$storage->attach($object)` on an `SplObjectStorage`.

A value is found with `==`. For an object of your own class, `==` is true only for the same object.
Two objects with equal property values are two different values, so the set keeps both. A
`Core\Time` value such as a `Date` is compared by its value, so the set keeps one `Date` for each day.
For a scalar such as an `int` or a `string`, equal values are the same value.

The set keeps the value alive while it is in it. A `foreach` over the set gives the values in the
order they were first added.

The examples show adding the same object twice, two equal objects as two members, and a common
use: finding the distinct customers in a list of orders.
