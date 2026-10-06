Returns the value of one cookie that the browser sent with the request. You give the name of the
cookie. The result is its value as a string, or `null` when the request has no cookie of that name.

The name must match exactly. Upper and lower case are different, and `a.b` and `a_b` are two
different cookies. The value is returned exactly as it arrived, and nothing is decoded.

When two cookies have the same name, the first one is returned. A cookie whose name starts with
`__Host-` can exist only once in a browser. So when two of them arrive, the result is `null`.

The result is tainted, which means it came from outside your program. A command-line program
answers no request, so `cookie` throws a `LogicError` there.
