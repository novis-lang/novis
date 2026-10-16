Deletes a key and its value from a `Core\ObjectMap`. After `remove`, `has` returns `false` for that
key, `get` returns `null`, and `count` is one lower. This replaces `unset($storage[$object])` on an
`SplObjectStorage`.

A key is matched by identity. An object with equal fields is a different key, so `remove` does not
delete it. If the key is not in the map, `remove` does nothing and does not throw an error.

The map stops keeping the key and the value alive. If nothing else uses them, they are freed.

The examples show removing one entry, removing a key that is not in the map, and a common use:
tracking the jobs that are still running.
