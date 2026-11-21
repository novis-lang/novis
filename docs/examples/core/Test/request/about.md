`Core\Test::request()` sends one request to your own program and returns the response. It opens no
network connection. For each request, Novis runs your program file again, from its first line. In
that run, the `Core\Request` methods read the request you sent.

You give the HTTP method and the path, such as `/orders/7`. Options set the rest: `headers`,
`cookies`, the client address `clientIp` and the `scheme`. A body is one of `body`, `json`, `form`
or `multipart`, the same options as in `Core\Http\Client`. A server can serve your program under a
path prefix, such as `/shop`. `mount` and `captures` set that prefix.

The result is a `Core\Test\Response`. `status()`, `body()`, `json()`, `headers()` and `cookies()`
return what the program sent. The response is the one the server sends. A write from another site
gets `403`, and a body over the size limit gets `413`. A `HEAD` request gets no body. A request that
throws gets `500`, and `error()` returns the class and message of the error.

**Good to know:** a program can start work that runs after the response is sent. The call returns
only when that work has ended. A request cannot send another request.
