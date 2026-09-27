Sets one header on the response. A header is a name and a value that the client reads before the
body, such as `Cache-Control: no-store`. The server already adds some headers of its own. If you set
one of them, your value replaces the server's value.

If you set the same name twice, the header has the last value. Names are compared without upper and
lower case, so `cache-control` and `Cache-Control` are the same header.

The name contains letters, digits and a few marks such as `-`. The value contains only printable
ASCII characters, so it cannot contain a newline. A newline would let a value start a second header.
You cannot set `Content-Type`: the body method you use sets it. For any of these, `setHeader` throws
a `LogicError`. Text from a visitor is not allowed in the name or the value, and does not compile.

The examples show a page that is not cached, a header that is not allowed, and the headers of an API
answer.
