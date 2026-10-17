Ends a response body that `Core\Response::stream` started. The client receives the end of the body at
once. Your program can go on running after `end()`, but it cannot write to the body again. A
`write()` after `end()` throws a `LogicError`. Calling `end()` a second time does nothing.

Without `end()`, the body ends when the program ends. A stream started with the `$mustEnd` argument
is different. Its body is complete only after `end()`. If the program stops before that, the body is
cut off, so the browser shows a failed download. The error log then names the stream with the
`$mustEnd` value. Use `$mustEnd` for a file download, where a part that is missing makes the whole
file useless.

The examples end a download, end a body before more work, and end a body with a known length.
