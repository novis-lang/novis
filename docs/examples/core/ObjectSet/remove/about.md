Takes a value out of a `Core\ObjectSet`. After `remove`, `has` returns `false` for that value and
the count is one less. `remove` returns nothing. This replaces `$storage->detach($object)` on an
`SplObjectStorage`.

A value is found with `==`. For an object of your own class, `remove` takes out only the same object
that was added. An object with equal property values is a different object, and the set does not
change. A `Core\Time` value such as a `Date` is compared by its value, so an equal `Date` is removed.
If the value is not in the set, `remove` does nothing and does not throw an error.

After `remove`, the set no longer uses the value. If nothing else uses it, it is freed.

The examples show the set before and after `remove`, a value that is not in the set, and a common
use: a list of users who are online, where a user is removed when they log out.
