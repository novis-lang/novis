Returns `true` when a heap has no values, and `false` when it has at least one.

`peek` and `pop` throw a `RuntimeError` on an empty heap, so `isEmpty` is the check to make before
you call them. A common pattern is a loop that runs while the heap is not empty and takes one value
out in each round. A new heap is empty, and a heap becomes empty again after its last value is taken
out.

**Good to know:** this replaces PHP's `SplHeap::isEmpty`.
