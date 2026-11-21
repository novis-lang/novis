In a test, `error()` returns the error that stopped a request, if your program threw one and did not
catch it.

`Core\Test::request()` sends a request to your own program. If the program throws an error that it
does not catch, the request fails. `error()` then returns the class and the message of that error,
as `Class: message`, for example `RuntimeError: the cart is empty`. If the request did not fail,
the result is `null`.

A failed request has the status `500` and an empty body, the same response a real visitor gets. From
`status()` and `body()`, a test only sees that the request failed. `error()` returns the reason.

**Good to know:** an error that the program catches does not stop the request, so `error()` returns
`null`.

**The examples below** show a request that fails and two requests with no error. The last one is a
test that checks the reason for a failure.
