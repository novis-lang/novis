Returns the value stored for a key in a `Core\ObjectMap`. If the map has no such key, `get`
returns `null`. It does not throw an error. This replaces reading an `SplObjectStorage` with
`$storage[$object]`.

A key is found with `==`. For an object of your own class, `==` is true only for the same object.
Another object with equal property values is a different key, so the result for it is `null`. A
`Core\Time` value such as a `Date` is compared by its value. A `Date` that you build again for the same
day finds the same entry.

The result type is `?V`, so you can write `$map->get($key) ?? $default` to give a default. A map
can also store `null` as a value. In that case `get` returns `null` too. Use `has` when you need to
know if the key is in the map.

The examples show a lookup, a missing key, and a common use: storing a computed result for each
object, so the program computes it only once. The last one finds a holiday with a `Date` read from
text.
