`trusted_proxies` lists the proxies whose forwarding headers the server reads.

A proxy in front of the server makes every connection come from the proxy. The proxy writes the
address of the real client in the `X-Forwarded-For` header, and `https` in `X-Forwarded-Proto`.
Any client can send these headers too, so the server reads them only from an address in this list.
An entry is one address, or a block of addresses such as `10.0.0.0/8`.

The list is empty by default, and the server then reads neither header. `Core\Request::clientIp()`
returns the address that connected, and `Core\Request::scheme()` returns `http`. When the list has
the address of your proxy, `clientIp()` returns the last address in `X-Forwarded-For` that is not
in the list. `scheme()` returns `https` when the proxy sent that.

**In plain words:** anybody can write any sender on an envelope. You believe the sender only when
a courier you know brought the letter, because that courier saw who gave it to them.

**Good to know:** `clientIp()` returns `null` when the proxy wrote `unknown` in place of the
address. You can always read both headers yourself with `Core\Request::header`.
