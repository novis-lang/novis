Returns the host of an address. The host is the name of the server, such as `example.com` in
`https://example.com/cart`.

`$uri->host()` returns the host as it was written. Upper-case letters stay upper case, so use
`Core\Str::lower` before you compare two hosts. An IPv6 address keeps its brackets, such as `[::1]`.
The port is not part of the host.

The result is `null` when the address has no `//` part, such as the link `/docs/start`. The result
is `""` when the `//` part is empty, as in `file:///tmp/notes.txt`.

**The examples below** read the host of a link, show the `null` and `""` results, and check that a
redirect only goes to your own site.
