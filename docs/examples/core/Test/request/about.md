`Core\Test::request()` sends one request to your own program and returns the response. It opens no
network connection. For each request, Novis runs your program file again, from its first line. In
that run, the `Core\Request` methods read the request you sent.

You give the HTTP method and the path, such as `/orders/7`. Options set the rest: `headers`,
`cookies`, the client address `clientIp` and the `scheme`. To send a body, use one of `body`, `json`,
`form` or `multipart`, the same options as in `Core\Http\Client`. A server can serve your program
under a path prefix, such as `/shop`. `mount` and `captures` set that prefix.

The result is a `Core\Test\Response`. `status()`, `body()`, `json()`, `headers()` and `cookies()`
return what the program sent. The response is the same one a real server sends. A form post from
another site gets `403`, and a body over the size limit gets `413`. A `HEAD` request gets no body. A
request that throws gets `500`, and `error()` returns the class and message of the error.

**Good to know:** a program can start work that runs after the response is sent. The call returns
only when that work has ended. Code that handles a request from `Core\Test::request` cannot call
`Core\Test::request` again. A file the program sends with
`Core\Response::sendFile` is read into the test's memory. If the file is larger than the memory the
test may still use, the test ends with a memory error.
