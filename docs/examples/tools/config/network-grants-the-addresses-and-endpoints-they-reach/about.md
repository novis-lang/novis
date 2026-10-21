A network grant names the hosts that a program may connect to and the endpoints that it may listen
on.

With no `connect` line, a program may connect to any public host. When you write `net.connect`,
only the hosts in the list are allowed. A host with a private, link-local or unspecified address is
always blocked. So a host name cannot point a request into your network.

Loopback (`127.0.0.1` and `::1`) is allowed from the command line and in development mode. For
another internal service, add its address to `internal`. An entry is one IP address or a range such
as `172.18.0.0/16`. When you write `connect`, the host must also be there.

`net.listen` is a list of `address:port` endpoints, and `true` allows every endpoint. Two ways to
write the same endpoint match. An invalid entry matches nothing.

**Good to know:** the shared cache needs neither grant. Its address is in `[cache.shared] url`, and
the `cache.shared` capability allows the connection.
