Returns the bytes of a message that `Core\Net\Datagram::receive` returned. The result is
`tainted bytes`, because the bytes come from the network. Check them before you use them in a
query, a file name or a web page.

An empty result is a real message, because a UDP message can have zero bytes. If the message was
longer than the `$max` you gave `receive`, the result has only the first `$max` bytes.

You can call `payload()` as often as you want. It returns the same bytes each time, also after the
socket is closed.
