Returns the value stored for a key in a `Core\ObjectMap`. If the map has no such key, `get`
returns `null`. It does not throw an error. This replaces reading an `SplObjectStorage` with
`$storage[$object]`.

A key is found by identity. `get` finds a value only when you give it the same object you used with
`set`. Another object with equal fields is a different key, so the result for it is `null`.

The result type is `?V`, so you can write `$map->get($key) ?? $default` to give a default. A map
can also store `null` as a value. In that case `get` returns `null` too. Use `has` when you need to
know if the key is in the map.

The examples show a lookup, a missing key, and a common use: storing a computed result for each
object, so the program computes it only once.
