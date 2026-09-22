Registers a function that runs when an error reaches the top of your program and no `catch` handles
it.

The function gets the error object itself, so it can read its message and check its class. Use it to
print a clear message for the person who ran the program, or to add a detail you know and the error
does not. It runs once. After it returns, the program still ends, the normal error report is still
written, and the exit status is still 1.

If the function throws an error itself, that error is ignored and the first one is reported.

**Good to know:** only one function is registered. A second call replaces the first one. For a
resource limit such as memory, use `Core\Fatal::onLimit` instead.
