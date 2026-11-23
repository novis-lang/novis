`Core\Test::request()` sends one request to your own program and returns the response. It opens no
network connection. Novis runs your program file again from its first line, and the
`Core\Request` methods read the request you sent.

You give the HTTP method and the path, such as `/orders/7`. Options set `headers`, `cookies`, the
client address `clientIp` and the `scheme`. The body options `body`, `json`, `form` and `multipart`
are the same as in `Core\Http\Client`. `mount` and `captures` set a path prefix, such as `/shop`.

The result is a `Core\Test\Response` with `status()`, `body()`, `json()`, `headers()` and
`cookies()`. It is the same response a real server sends. A request that throws gets `500`, and `error()` returns the class and message of the
error.

**Good to know:** the call returns only when all work the program started has ended. Code that
handles such a request cannot call `Core\Test::request` again. A file sent with
`Core\Response::sendFile` is read into the test's memory. If the file is larger than the memory the
test may still use, the test ends with a memory error.
