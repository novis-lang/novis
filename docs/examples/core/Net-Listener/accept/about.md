Waits for the next client to connect to a TCP server, and returns a `Core\Net\Stream` for that
client. The server reads from the stream and writes to it to talk to that one client.

`$within` is how long `accept` waits. If no client connects in that time, it throws a
`TimeoutError`. The server still works after a timeout, and you can call `accept` again. A client
that connects before `accept` is called waits in a queue, so `accept` returns at once.

`accept` needs no setting in `nvs.toml` of its own. The permission to listen was checked when
`Core\Net::listen` opened the server. Text a client sends is `tainted`, because it comes from the
network.

**Good to know:** closing the server does not close the clients it already accepted. Each client
has its own stream, and the request closes every stream it did not close itself when it ends.
