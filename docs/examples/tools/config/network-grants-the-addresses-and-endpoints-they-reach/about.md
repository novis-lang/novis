A network grant names the hosts that a program may connect to and the endpoints that it may listen
on.

`net.connect` is a list of host names. A granted host is still blocked when its address is a
loopback, private, link-local or unspecified address. An attacker who controls a host name can make
it resolve to an address inside your network, and this rule stops that. To reach an internal
service, add its IP address to `internal`. Each `internal` entry is one IP address. A host name, a
range and `true` are not allowed there. The entry alone grants nothing, because the host must also
be in `connect`.

`net.listen` is a list of `address:port` endpoints, and `true` allows every endpoint. Novis
compares the endpoint and not the text, so two ways to write one endpoint match. An entry that is
not a valid endpoint matches nothing.

**Good to know:** the shared cache needs neither grant. The person who runs the server writes its
address in `[cache.shared] url`, and the `cache.shared` capability allows the connection.
