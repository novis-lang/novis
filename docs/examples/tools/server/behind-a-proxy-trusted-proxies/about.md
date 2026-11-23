`trusted_proxies` lists the proxies whose forwarding headers the server reads.

Behind a proxy, every connection comes from the proxy. The proxy writes the address of the real
client in the `X-Forwarded-For` header, and `https` in `X-Forwarded-Proto`. Any client can send
these headers too, so the server reads them only from an address in this list. An entry is one
address, or a block such as `10.0.0.0/8`.

The list is empty by default. Then `Core\Request::clientIp()` returns the address that connected,
and `Core\Request::scheme()` returns `http`. For a proxy in the list, `clientIp()` returns the last
address in `X-Forwarded-For` that is not in the list. It returns `null` when the proxy wrote
`unknown`. `scheme()` returns `https` when the proxy sent that.

In `development` mode, the server checks the `Host` header of each request from a proxy on this
machine. It does this whether or not the proxy is in the list. If your proxy sends its own host
name, such as `app.test`, add that name to `[server] development_hosts`. Otherwise each request gets
`421`, and the response says which host to add.
