A network grant names the hosts that a program may connect to and the endpoints that it may listen
on.

With no `connect` line, a program may connect to any public host. `net.connect` is a list of host
names, and when you write it, only those hosts are allowed. A host is still blocked when its address
is a private, link-local or unspecified address. An attacker who controls a host name can make it
resolve to an address inside your network, and this rule stops that.

Loopback (`127.0.0.1` and `::1`) is allowed from the command line and in development mode. To reach
any other internal service, add its address to `internal`. An `internal` entry is one IP address or
a range such as `172.18.0.0/16`. A host name and `true` are not allowed there. When you write
`connect`, the host must also be in it.

`net.listen` is a list of `address:port` endpoints, and `true` allows every endpoint. Novis
compares the endpoint and not the text, so two ways to write one endpoint match. An entry that is
not a valid endpoint matches nothing.

**Good to know:** the shared cache needs neither grant. The person who runs the server writes its
address in `[cache.shared] url`, and the `cache.shared` capability allows the connection.
