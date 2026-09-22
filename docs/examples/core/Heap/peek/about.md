Returns the smallest value in a heap and leaves it in the heap.

`peek` gives the same value that `pop` would give next, but the heap does not change. Use it when
you want to decide something before you take the value out, for example whether the next job is due
yet. "Smallest" means the order the heap uses: its comparator function, the `compareTo` method of
its objects, or the normal order of numbers and strings.

**Good to know:** `peek` on an empty heap throws a `RuntimeError`. It does not return `null`. Call
`isEmpty` first when the heap can be empty. This replaces PHP's `SplHeap::top`.
