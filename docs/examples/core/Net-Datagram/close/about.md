Closes a UDP socket. The port is free again, and other programs can use it.

After `close`, every method of the socket throws a `RuntimeError`, and so does a second `close`.
Messages you already received stay readable, because each `Core\Net\Datagram\Message` has its own
copy of what arrived.

If your program does not call `close`, the socket is closed when the request ends. Call it yourself
when you are done with a socket, so the port is free sooner. A `finally` block is a good place for
it, because it runs even when an error is thrown.
