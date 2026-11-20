Two interfaces make a class walkable by a `foreach` loop that gets values: `Iterator<T>` and
`Iterable<T>`. A loop that gets a key with each value uses `KeyedIterator<K, V>` and
`KeyedIterable<K, V>`.

`Iterator<T>` is the cursor. `advance()` moves it to the next element and returns `false` once there
are none left. `current()` returns the element the last `advance()` moved to. `Iterable<T>` has one
method, `iterate()`, which returns a fresh `Iterator<T>`. A class declares one of the two at a
concrete type, such as `Iterator<int>`. Both names also work as a parameter type or a variable type.

**Good to know:** `foreach` calls `iterate()` once per loop, so one `Iterable<T>` object can be
walked twice, or nested inside a loop over itself. An `Iterator<T>` has no keys, so a loop over one
cannot bind a key. To give keys, implement `KeyedIterable<K, V>`.

**The examples below** show a cursor written by hand. Then a class gives out a new cursor for each
loop, and a search result is read page by page. Example 04 shows that a loop over a cursor cannot get
a key.
