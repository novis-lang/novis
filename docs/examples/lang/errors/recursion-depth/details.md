The message of the error is `the call stack is too deep`. `$e is RuntimeError` is `true`.

A function with a few parameters and locals reaches fewer calls than a function with one. The exact
number can change between versions of Novis, so do not write code that needs it.

A `catch` in any of the calls handles the error. The calls inside it are gone, and the calls outside
it go on as usual. The program may call the deep function again after a `catch`, and it throws a
`RecursionError` again.

`finally` blocks in each call run while the error leaves the calls.

`backtrace` has one entry for each call, so a `RecursionError` has a few thousand entries. An
uncaught one prints all of them in the report.

You may throw a `RecursionError` yourself with `new RecursionError("…")`.
