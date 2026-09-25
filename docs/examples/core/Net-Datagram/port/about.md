Returns the port number a UDP socket is bound to. This replaces the port part of PHP's
`stream_socket_get_name`.

You usually open a socket with port `0`, which means "pick any free port". The operating system then
picks the port, and `port()` is how your program finds out which one it is. You give that number to
the program that should send messages to this socket.

The result is always between 1 and 65535. Calling `port()` on a socket that is already closed
throws a `RuntimeError`.
