A class implements `Iterator<T>` or `Iterable<T>` so that a `foreach` loop can get values from its
objects. A loop that gets a key with each value uses `KeyedIterator<K, V>` and `KeyedIterable<K, V>`.

`Iterator<T>` is a cursor: an object that remembers its position in a sequence. `advance()` moves it
to the next element and returns `false` once there are none left. `current()` returns the element the
last `advance()` moved to. `Iterable<T>` has one method, `iterate()`, which returns a new `Iterator<T>`.
A class implements one of the two with a concrete type, such as `implements Iterator<int>`. You can
also use `Iterator<T>` and `Iterable<T>` as a parameter type or a variable type.

**Good to know:** `foreach` calls `iterate()` once per loop, so one `Iterable<T>` object can be
walked twice, or nested inside a loop over itself. An `Iterator<T>` has no keys, so a loop over one
cannot get a key. To give keys, implement `KeyedIterable<K, V>`.

**The examples below** show a cursor written by hand. Then a class gives out a new cursor for each
loop, and a search result is read page by page. Example 04 shows that a key variable over a cursor
does not compile.
