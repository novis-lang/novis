Returns how many values a heap has.

Every `push` adds one to the count and every `pop` removes one. A value that was added twice is
counted twice. A new heap has a count of `0`. The count is a `uint`, a whole number that is never
negative.

**Good to know:** to check whether a heap is empty, `isEmpty` says it more clearly than comparing the
count with `0`. This replaces PHP's `SplHeap::count`.
