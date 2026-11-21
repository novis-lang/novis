`Core\Test::request()` sends one request to your own program and returns the response. It opens no
network connection. Novis runs your program again from the top, and the `Core\Request` methods read
the request you sent.

You give the HTTP method and the path, such as `/orders/7`. Options set the rest: `headers`,
`cookies`, the client address `clientIp` and the `scheme`. A body is one of `body`, `json`, `form`
or `multipart`, the same options as in `Core\Http\Client`. `mount` and `captures` set the prefix
that the server mounts your program under.

The result is a `Core\Test\Response`. `status()`, `body()`, `json()`, `headers()` and `cookies()`
return what the program sent. The response is the one the server sends. A write from another site
gets `403`, and a body over the size limit gets `413`. A `HEAD` request gets no body. A request that
throws gets `500`, and `error()` returns the class and message of the error.

**Good to know:** the call returns after the program's after-response work has ended. A request
cannot send another request.
