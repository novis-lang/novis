`bytes()` returns the body of the response to a request that `Core\Test::request()` sent. The
result has the type `bytes`, which is raw binary data. So it works for every body: text, a PDF, an
image, or a file sent with `Core\Response::sendFile`. If the program wrote nothing, the result is
empty `bytes`.

`body()` returns the same body as a string. It throws a `RuntimeError` when the body is not valid
UTF-8. Use `bytes()` for such a body. `Core\Test::assertEquals` compares it with the bytes you
expect.

A request fails when the program throws an error that it does not catch. Then the result is empty
`bytes`, because the server sends no body for a failed request. A test gets the same response as a
real visitor. A `HEAD` request
also gives empty `bytes`.

You can call `bytes()` as often as you like. Each call returns the same bytes.

**The examples below** show how to read an image, how to compare a download with the file you
expect, and what a failed request returns.
