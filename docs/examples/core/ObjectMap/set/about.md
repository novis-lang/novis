Stores a value for a key in a `Core\ObjectMap`. If the key is new, `set` adds it at the end of
the map. If the key is already in the map, `set` replaces its value and the key keeps its place.
This replaces `$storage[$object] = $value` on an `SplObjectStorage`.

A key is matched by identity. Two objects with equal fields are two different keys, so each one
gets its own entry. The value can be `null`, and the key is still in the map.

The map keeps the key and the value alive while they are in it. `remove` and `clear` free them
again.

The examples show adding and replacing a value, two equal objects as two keys, and a common use:
counting how many times each object appears.
