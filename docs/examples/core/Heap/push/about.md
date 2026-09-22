Adds a value to a heap. A heap keeps its values in order, so the smallest value is always ready to
take out.

A heap is also called a priority queue. `push` puts the new value in its right place, and this stays
fast even when the heap has many values. If you add the same value twice, the heap keeps both. The
heap decides what "smallest" means in this order: the comparator function you gave to
`new Core\Heap`, then the `compareTo` method of objects that implement `Comparable`, then the normal
order of numbers and strings.

**In plain words:** a heap is like a pile of tasks where the most urgent task is always on top. You
can add a task at any time, and it goes to its right place in the pile.

**Good to know:** if two values cannot be compared, `push` throws a `RuntimeError`. This replaces
PHP's `SplPriorityQueue::insert` and `SplMinHeap::insert`.
