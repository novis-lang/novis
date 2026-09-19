`throw $e;` inside a `catch` sends the same error on to the next handler.

The object does not change. Its `message`, its `previous` and its class stay as they were. Only
`location` moves to the line of the rethrow, so a log line written later says where the error was
sent on. Rethrow when the code you are in has something to do first, such as closing a file or
counting a failure, and cannot handle the error itself.

**Good to know:** to add information of your own, do not rethrow. Throw a new error and give it the
caught one as its cause, with `{previous: $e}`. A handler further out then has both messages.

**The examples below** show a plain rethrow, a rethrow after a log line, and a retry that gives up
and sends the last error on.
