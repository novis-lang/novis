`Core\ObjectSet<T>`, `Core\Heap<T>` and `Core\Task\Channel<T>` are each an `Iterable<T>`.
`Core\ObjectMap<K, V>` is an `Iterable<K>`. You can pass each one to a parameter of that type.

A loop over a map gives the keys, never the values. `int $v` over an `ObjectMap<string, int>` is the
error `E0401`. A key variable is not allowed: `foreach ($map as string $k => int $v)` is the error
`E0444`. The written type of the loop variable must be the element type, or the error is `E0401`.
`var $v` takes the element type.

A heap made with a compare function gives its elements in the order that function sets. A heap of
`Comparable` objects gives them in the order of `compareTo`.

A set, a map and a heap can be walked many times, and each loop gives every element again.

A loop over a channel takes each value out of it. A second loop over the same closed channel gets no
values. Values sent before `close` are still given to the loop.
