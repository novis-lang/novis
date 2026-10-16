`Core\Task::map` calls one function once per element of an array, each call at the same time as the
others, and gives you the results as an array.

The results come back under the input's own keys and in the input's own order. The order the jobs
finished in does not change anything you see. The function receives the value and the key, in that
order, and you may write only the value if the key is not needed. The result array holds whatever
type the function returns.

`Core\Task::map` returns only when every element has been handled, so nothing from the group is
still running afterwards. If one job throws, the rest are stopped and the error reaches you at the
call.

**The examples below** show one job per element, a function that reads the key as well, and an
import that checks every row of a file at the same time.
