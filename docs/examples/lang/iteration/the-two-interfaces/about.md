Two interfaces make a class walkable by a `foreach` loop: `Iterator<T>` and `Iterable<T>`. There are
no others.

`Iterator<T>` is the cursor. `advance()` moves it to the next element and returns `false` once there
are none left. `current()` returns the element the last `advance()` moved to. `Iterable<T>` has one
method, `iterate()`, which returns a fresh `Iterator<T>`. A class declares one of the two at a
concrete type, such as `Iterator<int>`. Both names also work as a parameter type or a variable type.

**Good to know:** `foreach` calls `iterate()` once per loop, so one `Iterable<T>` object can be
walked twice, or nested inside a loop over itself. A cursor has no keys, so a loop over one cannot
bind a key.

**The examples below** show a cursor written by hand, then a class that gives out a fresh cursor per
loop, then a search result read page by page.
