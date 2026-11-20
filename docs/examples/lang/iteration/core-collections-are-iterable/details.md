`Core\ObjectSet<T>`, `Core\Heap<T>` and `Core\Task\Channel<T>` are each an `Iterable<T>`.
`Core\ObjectMap<K, V>` is an `Iterable<K>` and a `KeyedIterable<K, V>`. You can pass each collection to
a parameter with one of its types.

A loop with only a value variable over a map gives the keys. A loop with a key variable gives each key
with its value. Over an `ObjectMap<Tag, int>`, `foreach ($map as var $tag => var $n)` gives `$tag` the
type `Tag` and `$n` the type `int`. If you write a type for a variable and it does not fit, the error
is `E0401`. A set, a heap and a channel have no keys, so a key variable over one is the error `E0444`.

A heap made with a compare function gives its elements in the order that function sets. A heap of
`Comparable` objects gives them in the order of `compareTo`.

A set, a map and a heap can be walked many times, and each loop gives every element again.

A loop over a channel takes each value out of it, so a second loop over the same closed channel gets no
values. The first loop also gets the values that were sent before `close`.
