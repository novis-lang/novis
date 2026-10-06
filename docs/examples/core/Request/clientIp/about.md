Returns the network address of the client that sent the request, as a string such as
`203.0.113.7` or `2001:db8::1`. An IPv6 address is always in its short form.

Your server may run behind a proxy or a load balancer. Then the address comes from the
`X-Forwarded-For` header, but only when that proxy is listed in `[server] trusted_proxies`.
Otherwise Novis ignores the header and gives the address of the connection itself. This way a
client cannot choose its own address.

The result is `null` when the request has no address, for example over a Unix socket. The result
is tainted, which means it came from outside your program. A command-line program answers no
request, so `clientIp` throws a `LogicError` there.
