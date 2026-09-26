Removes every value from a `Core\ObjectSet`. After `clear`, the count is `0` and `isEmpty`
returns `true`. `clear` returns nothing.

The set itself is still there, so you can add values to it again. Every variable that points to the
set sees the empty set.

`clear` frees the values the set kept alive. An object that nothing else uses is deleted at that
point. Clearing a set that is already empty does nothing.

The examples show a set before and after `clear`, adding values again after `clear`, and a common
use: one set that is emptied before each batch of work.
