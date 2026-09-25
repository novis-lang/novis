Returns the port of the program that sent a message. Together with `host()`, it is the address
where a reply goes. The result is always between 1 and 65535.

Every open socket has its own port. So when two programs on the same computer send to you, the
host is the same for both, and the port tells them apart.

The port is the port of the sender's socket, not of your own socket. Your own port is
`Core\Net\Datagram::port`.
