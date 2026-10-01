`header()` returns the value of one header of the answer to a request that `Core\Test::request()`
sent. Upper and lower case in the name do not matter. If the answer has no header of that name, the
result is `null`.

These are the headers your program sends to a visitor. `Content-Type` is always there. If the
program did not set it, its value is `text/html; charset=utf-8`. A redirect sets `Location`, and
`Core\Response::setHeader` sets other headers. The server also adds headers of its own, such as
`Content-Length` and `Date`. `header()` does not return those.

When a header has more than one line, `header()` joins the values with `, `. Each cookie has its
own `Set-Cookie` line, and two joined cookies are not a cookie. So `header("Set-Cookie")` throws a
`LogicError`. Use `cookies()` or `headers()` to read cookies.

A request that failed has no headers, so the result is always `null`.

**The examples below** show how to read the content type, how to test a redirect, and what
`header()` does with `Set-Cookie`.
