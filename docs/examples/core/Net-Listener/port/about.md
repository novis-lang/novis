Returns the port number a TCP server is listening on. This replaces PHP's `stream_socket_get_name`
for a server.

You need it when you start a server with port `0`. Port `0` tells the operating system to pick any
free port, and `port` tells you which one it picked. A client needs that number to connect. Tests
use port `0` often, because two tests that run at the same time then never ask for the same port.

The result is a number from 1 to 65535. It stays the same until you close the server. After
`close`, `port` throws a `RuntimeError`. A server that `Core\Net::listenLocal` started on a socket
file has no port, so `port` throws a `RuntimeError` there too.
