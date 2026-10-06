Removes the smallest value from a heap and returns it.

After `pop`, the next smallest value moves to the front, so calling `pop` again and again gives every
value in order. This stays fast even when the heap has many values. By default the smallest value
comes first. To get the largest value first, give `new Core\Heap` a comparator function that
compares the other way round.

**In plain words:** a heap is like a waiting room where the most urgent patient is always called
next, whatever order the patients arrived in.

**Good to know:** `pop` on an empty heap throws a `RuntimeError`. Call `isEmpty` first when the heap
can be empty.
