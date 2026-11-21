`body()` returns the body of the response as a string. `Core\Test::request()` sends a request to
your own program in a test, and the program answers it. The body is everything the program wrote
with `echo`, in order. When the program used a method such as `Core\Response::json`, the body is
what that method sent. If the program wrote nothing, the result is an empty string.

A request fails when the program throws an error that it does not catch. Then the result is also
an empty string, because the server sends no body for a failed request. A test gets the same
response as a real visitor. A `HEAD` request also gives an empty string.

You can call `body()` as often as you like. Each call returns the same text.

**The examples below** show how to read a written page, what a failed request returns, and how to
test the JSON that an API sends.
