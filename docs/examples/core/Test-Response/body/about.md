`body()` returns the text your program wrote while it answered one request that
`Core\Test::request()` sent. The text is everything the program wrote with `echo`, in order, or
the body that a method such as `Core\Response::json` sent. If the program wrote nothing, the result
is an empty string.

If the program throws an error that it does not catch, the result is also an empty string. The
server sends no body for a failed request, and a test sees the same answer as a real visitor.

You can call `body()` as often as you like. Each call returns the same text.

**The examples below** show how to read a written page, what a failed request returns, and how to
test the JSON that an API sends.
