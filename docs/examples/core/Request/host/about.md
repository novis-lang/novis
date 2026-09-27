Returns the host name the request was sent to, such as `shop.example.com`. The value comes from the
`Host` header of the request.

The result is always in lower case. A port such as `:8443` is removed, and so is one dot at the end
of the name. So `Shop.Example.com.:8443` returns `shop.example.com`. An IPv6 address keeps its
brackets, for example `[2001:db8::1]`. The result is `null` when the request has no `Host` header.

`host` reads only the `Host` header. Headers such as `X-Forwarded-Host` are never read, because any
client can send them. If you need the port, `Core\Request::header("host")` returns the header
exactly as it arrived.

The result is tainted, which means it came from outside your program. A command-line program
answers no request, so `host` throws a `LogicError` there.

This replaces `$_SERVER['HTTP_HOST']` in PHP.
