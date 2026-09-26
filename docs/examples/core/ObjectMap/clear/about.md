Removes every entry from a `Core\ObjectMap`. After `clear`, `count` returns `0` and `isEmpty`
returns `true`.

`clear` does not delete the objects you used as keys or values. Other variables that refer to them
still work. Only the map forgets them. If nothing else refers to an object, its memory is freed.

The map itself stays usable. You can call `set` on it again right away, and a second `clear` on an
empty map does nothing.

A common use is a cache for one batch of work. The program fills the map while it handles the batch,
and calls `clear` before the next batch starts. The examples show this, and they show that the keys
still exist after `clear`.
