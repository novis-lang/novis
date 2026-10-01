`headers()` returns every header of the answer to a request that `Core\Test::request()` sent. The
result is an `array<array<string>>`. Each key is a header name in lower case. Each value is a list
with one entry for each line of that header, in order.

Most headers have one line. `Set-Cookie` has one line for each cookie the program set. To check a
whole cookie line, with attributes such as `Path` and `HttpOnly`, read `headers()["set-cookie"]`.

These are the headers your program sends to a visitor. The first key is always `content-type`. The
server also adds headers of its own, such as `Content-Length` and `Date`. `headers()` does not
return those. A request that failed has no headers, so the result is an empty array.

**The examples below** show every header of a page, the lines of two cookies, and a request that
fails.
