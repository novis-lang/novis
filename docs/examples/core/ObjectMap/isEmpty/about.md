Checks if a `Core\ObjectMap` has no entries. It returns `true` when the map is empty and `false`
when it has at least one key. `$map->isEmpty()` gives the same answer as `$map->count() == 0`,
and it is shorter to read.

`isEmpty` counts keys, not values. A key whose value is `null` is still an entry, so a map with
only that key is not empty. The map is empty again after `remove` takes out its last key, or
after `clear`.

The examples show a new map, a map with a `null` value, and a common use: saving changed records
at the end of a request, only when there is something to save.
