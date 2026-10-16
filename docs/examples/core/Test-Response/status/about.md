`status()` returns the HTTP status code of the answer to one request that `Core\Test::request()`
sent to your own program. It is the code the program set with `Core\Response::setStatus`, or the
code a method such as `Core\Response::redirect` set.

If the program set no code, the result is `200`. If the program throws an error that it does not
catch, the result is `500`. These are the same codes the server sends to a real visitor, so a test
can check an error page as well as a normal one.

**The examples below** show a request with no status, a status the program sets, and a test that
checks that an old address redirects to a new one.
