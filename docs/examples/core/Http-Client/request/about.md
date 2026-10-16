Sends an HTTP request with a method your program chooses while it runs, and returns the server's answer.

`Core\Http\Client` has one function for each common method, such as `get`, `post` and `delete`.
`request` takes the method as its first argument, a `Core\Http\Method` value, so the method can come
from a variable or a list. It also sends `OPTIONS` and `TRACE`, which have no function of their own.
It takes the same options as the other functions and returns the same `Core\Http\Response`.

`request` checks two things just before it sends. A `GET` or `HEAD` request with a body throws a
`RuntimeError`. A `POST` or `PATCH` that is sent again can do the same thing twice, so a retry
without `retryIdempotencyKey` also throws a `RuntimeError`.

**Good to know:** the examples run without a network, so every request in them is refused before it
is sent. They show an `OPTIONS` request, the two checks, and a job that sends saved changes.
