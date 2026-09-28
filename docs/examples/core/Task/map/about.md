`Core\Task::map()` calls one function for each element of an array, and runs the calls at the same
time. It waits until every call has finished.

The result is an array with the same keys, in the same order, as the array you give it. The order
does not change when a later call finishes first. Your function gets the value and, if it declares
a second parameter, the key as a `string`. Use it when a program has a list of slow jobs of the same
kind, such as checking many web addresses or loading many files.

If one call throws an error, the other calls are stopped, and `map` throws that error. The option
`limit` sets how many calls run at the same time. The option `deadline` is a time limit for the
whole call. When it runs out, the call throws a `TimeoutError`.

**The examples below** show that the order stays the same, a function that uses the key, and many
links checked with a limit and a time limit.
