Closes an open file. After `close`, the handle cannot be used any more.

`$file->close()` closes the `Core\IO\File` that `Core\IO::open` returned. Everything you wrote
through the handle is in the file when `close` returns. A lock that the handle took with `lock`
is released at the same time.

Calling `close` is optional. A handle that is still open when the request or the program ends is
closed for you. Call `close` yourself when you are done with a file and the program continues,
for example inside a loop that opens many files.

Every method you call on a closed handle throws a `RuntimeError`. This includes a second `close`.
The error message names the method and the path of the file.
