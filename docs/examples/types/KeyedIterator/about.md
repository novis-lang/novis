The interface for a cursor that moves through entries one at a time, each with a key and a value.

A `KeyedIterator<K, V>` has three methods. `advance` moves to the next entry and returns `true`. If there
is no next entry, it returns `false`. `key` returns the key of the entry that `advance` just
moved to, and `current` returns its value. You write the types when you declare the class, for example
`implements KeyedIterator<int, string>`. A class without type arguments does not compile.

A `foreach` loop can use a cursor directly: `foreach ($cursor as var $key => var $value)`. The loop
calls `advance` before each step and stops when it returns `false`. A cursor is used only once. A
second loop over the same cursor visits nothing. If you need to walk the entries more than once, write
a class that implements `KeyedIterable` and returns a new cursor each time.

**Good to know:** `key` and `current` throw a `LogicError` before the first `advance` and after
`advance` returns `false`. A class that does not have all three methods does not compile, with the
error `E0449`.
