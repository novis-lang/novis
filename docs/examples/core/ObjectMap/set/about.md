Stores a value for a key in a `Core\ObjectMap`. If the key is new, `set` adds it at the end of
the map. If the key is already in the map, `set` replaces its value and the key keeps its place.
This replaces `$storage[$object] = $value` on an `SplObjectStorage`.

A key is found with `==`. For an object of your own class, `==` is true only for the same object.
Two objects with equal property values are two different keys, so each one gets its own entry. A
`Core\Time` value such as a `Date` is compared by its value, so `set` with an equal `Date` replaces
the value. A `DateTime` is compared by its moment. The same moment in two time zones is one key.

The value can be `null`, and the key is still in the map.

The map keeps the key and the value alive while they are in it. `remove` and `clear` free them
again.

The examples show adding and replacing a value, two equal objects as two keys, and a common use:
counting how many times each object appears.
