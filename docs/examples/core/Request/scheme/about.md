Returns the scheme of the request that your program is answering. The scheme is `"http"` or
`"https"`. It tells you if the connection was encrypted. The result is never `null`, because
every request arrives over one of the two.

Often a proxy receives the encrypted connection and sends the request on to your server over
plain `http`. The proxy then adds the header `X-Forwarded-Proto: https`. Novis believes this
header only when the server configuration lists the proxy in `trusted_proxies`. Without that
setting, `scheme` returns the scheme of the connection itself, so a client cannot claim `https`
by sending the header.

The result is tainted. A tainted string came from the client, so Novis makes you check it
before you use it in a query or a file path. A command-line program answers no request, so
`scheme` throws a `LogicError` there.

**The examples below** show the full address of a page, a header that `scheme` does not believe,
and a login page that is only shown over `https`.
