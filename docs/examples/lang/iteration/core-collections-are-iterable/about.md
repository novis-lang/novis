A `foreach` loop walks a `Core` collection the same way it walks an array.

Four classes work this way. `Core\ObjectSet` gives you its members and `Core\ObjectMap` gives you its
keys, both in the order you added them. `Core\Heap` gives you its elements smallest first, and leaves
the heap full. `Core\Task\Channel` gives you each value another task sent, and the loop ends when that
task calls `close`.

A set, a map and a heap take a copy of their contents when the loop starts. The loop walks that copy,
so your code may add or remove inside the loop body. The loop still visits what was there at the
start, and your changes are in the collection when it ends.

**The examples below** show a set and a map first, then a heap, then a channel between two tasks.
