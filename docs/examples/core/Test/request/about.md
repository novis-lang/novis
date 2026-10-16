`Core\Test::request()` sends one request to your own program and returns the response. It opens no
network connection. The request runs through the same route table and the same code as a real
request. You give the HTTP method and the path, such as `Core\Http\Method::Get` and `/orders/7`. You
can also give `headers`, a `body` and a `mount` prefix. If the server mounts your program with a
pattern such as `/shops/{1}`, `captures` gives the parts of the prefix that the pattern matched.
`captures` without `mount` throws a `LogicError`.

The result is a `Core\Test\Response`. `status()` returns the status code. If the program set no
status, it is `200`. `body()` returns the text the program wrote, and `json()` and `jsonAs()` decode
it. `header()`, `headers()` and `cookies()` return the headers and cookies that the program set.

Novis runs your program again from the top to handle the request. In that run, the `Core\Request`
methods read the request you sent. A request cannot send another request, so in that run
`Core\Test::request` throws a `RuntimeError`.
