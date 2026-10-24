`bytes()` returns the body your program wrote while it answered one request that
`Core\Test::request()` sent. The result is `bytes`, so it works for every body: text, a PDF, an image,
or a file sent with `Core\Response::sendFile`. If the program wrote nothing, the result is empty
`bytes`.

`body()` returns the same body as a string. It throws a `RuntimeError` when the body is not valid
UTF-8. Use `bytes()` for such a body, and compare it with the bytes you expect.

If the program throws an error that it does not catch, the result is empty `bytes`. The server sends
no body for a failed request, and a test sees the same answer as a real visitor.

You can call `bytes()` as often as you like. Each call returns the same bytes.

**The examples below** show how to read an image, how to compare a download with the file you
expect, and what a failed request returns.
