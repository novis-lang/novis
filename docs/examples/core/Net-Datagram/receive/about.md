Waits for one message on a UDP socket and returns it as a `Core\Net\Datagram\Message`. The
message has three parts: the bytes that arrived, and the host and the port of the program that sent
them. You use that host and port to send a reply. This replaces PHP's `stream_socket_recvfrom`.

`$max` is the largest number of bytes you want to keep. A longer message is cut at `$max`, and the
rest of it is lost. No UDP message is larger than 64 KiB, so `receive` never keeps more than that.

`$within` is how long `receive` waits. If no message arrives in that time, it throws a
`TimeoutError`. The bytes are `tainted`, because they come from the network. Receiving needs no
setting in `nvs.toml` of its own. The permission to open the socket is enough.
