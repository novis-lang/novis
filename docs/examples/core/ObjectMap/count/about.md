Returns the number of entries in a `Core\ObjectMap`. The result is a `uint`, and it is `0` for an
empty map. This replaces PHP's `count()` on an `SplObjectStorage`.

Each key is counted once. When you call `set` with a key that is already in the map, the value is
replaced and the count stays the same. `remove` makes the count one smaller, and `clear` makes it
`0`.

A key is found by identity. Two different objects are two keys, even when all their fields are
equal. The examples show this, and show how to use `count` to limit how many tasks run at the same
time.

`count` does not walk the map, so it is fast even for a large map.
