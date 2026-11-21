`status()` returns the HTTP status code of a response. `Core\Test::request()` sends the request to
your own program in a test. The status code is the one the program set with
`Core\Response::setStatus`. A method such as `Core\Response::redirect` also sets a code.

If the program set no code, the result is `200`. If the program throws an error that it does not
catch, the result is `500`. The server sends the same codes to a real visitor. So a test can check
an error page as well as a normal one.

Some requests are stopped by the server before your program runs, and a test gets the same code. A
form post that another website forged gets `403`. A body over the size limit in the `[limits]` block
of `nvs.toml` gets `413`.

**The examples below** show a request with no status and a status the program sets. The last one is a test that
checks that an old address redirects to a new one.
