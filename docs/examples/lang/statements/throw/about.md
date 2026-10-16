`throw` raises an error. The method stops at that line and Novis looks for a `catch` clause that
matches, first in the same method and then in each caller.

Everything you throw is a `Throwable`, or a class that extends one. `throw new RuntimeError("the
reason");` is the usual form: the message is the first argument, and a class may add more.

If no `catch` matches, the program ends with status 1. It writes the message, the error class and
the methods that were running to standard error, as one log record.

`throw` is also an expression, so you can write it where a value is expected. `$name ?? throw new
LogicError("no name")` gives the name when there is one, and raises the error when there is not.

**Good to know:** `{previous: $e}` as a last argument records the error that caused this one. The
error you add then keeps the first one with it.
