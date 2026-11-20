A `foreach` loop walks a `Core` collection the same way it walks an array.

Four classes work this way. `Core\ObjectSet` gives you its elements in the order you added them.
`Core\ObjectMap` gives you its keys in the same order. Write `foreach ($map as var $key => var $value)`
to get each key with its value. `Core\Heap` is a collection that sorts its elements. It gives you its
elements smallest first, and they stay in the heap. `Core\Task\Channel` is a queue between two tasks.
It gives you each value the other task sent, and the loop ends when that task calls `close`.

A set, a map and a heap take a copy of their contents when the loop starts. The loop walks that copy,
so your code may add or remove elements inside the loop body. The loop still visits what was there at
the start, and your changes are in the collection when it ends.

**The examples below** show a set and a map first, then a heap, then a channel between two tasks.
