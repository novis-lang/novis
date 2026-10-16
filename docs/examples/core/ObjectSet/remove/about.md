Takes a value out of a `Core\ObjectSet`. After `remove`, `has` returns `false` for that value and
the count is one less. `remove` returns nothing. This replaces `$storage->detach($object)` on an
`SplObjectStorage`.

A value is matched by identity. `remove` takes out only the same object that was added. An object
with equal fields is a different object, and the set does not change. If the value is not in the
set, `remove` does nothing and does not throw an error.

The set stops keeping the value alive. An object that nothing else uses is deleted at that point.

The examples show the set before and after `remove`, a value that is not in the set, and a common
use: a list of users who are online, where a user is removed when they log out.
