Checks if a `Core\ObjectMap` contains a key. It returns `true` or `false`. This replaces
`isset($storage[$object])` and `$storage->contains($object)` on an `SplObjectStorage`.

A key is found by identity. `has` returns `true` only for the same object you used with `set`.
Another object with equal fields is a different key, so the result for it is `false`.

`has` checks the key, not the value. A key whose value is `null` is still in the map, so `has`
returns `true` for it. `get` returns `null` for that key and for a missing key, so `has` is the way
to tell the two apart. After `remove` or `clear`, `has` returns `false`.

The examples show a simple check, a key whose value is `null`, and a common use: doing a piece of
work only once for each object.
