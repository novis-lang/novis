Returns the value of one header of the request. You give the name of the header. The result is its
value as a string, or `null` when the request has no header of that name.

Upper and lower case in the name do not matter, so `Content-Type` and `content-type` return the same
value. A header that arrived empty returns an empty string.

A request can have the same header on more than one line. Then the values are joined into one
string, with `, ` between them, in the order they arrived. `Core\Request::headers` returns each line
on its own.

The result is tainted, which means it came from outside your program. A command-line program
answers no request, so `header` throws a `LogicError` there.

This replaces the `HTTP_` entries of PHP's `$_SERVER`.
