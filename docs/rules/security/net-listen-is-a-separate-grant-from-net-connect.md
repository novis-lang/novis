Which network capability an opening asks is decided by **what the program is doing**, not by which
transport it does it over. Reaching outward is `net.connect`, asked at `Scope::Host` and carrying
`rule:security/net-address-policy`'s denied-range table. Binding an endpoint is `net.listen`, asked of
the endpoint and carrying no address policy. Opening or binding a socket path is `net.local`
(`rule:config/net-local-is-named-and-not-on-the-roster`). **None of the three widens either of the
others**: a program that may reach a host may not bind one, and a program that may open a local socket
may not reach the network.

One socket can need two of them, because it does two things. A `Core\Net` datagram socket asks
`net.listen` once for its local port and `net.connect` for **every destination it sends to**, resolved
and checked at the send exactly as a TCP connect is checked at the connect. Granting it once at the
bind instead would make UDP a way around the address policy entirely: a program holding only its own
port could send to any address in any denied range.

`net.listen`'s entries are `address:port` literals matched exactly, and `true` is "any endpoint this
process may bind". There is no denied-range table here because the policy's terms **invert** under a
bind: binding loopback is the contained case and binding the unspecified address is the exposed one,
so importing the outbound table would deny the safe spelling and permit the dangerous one. What a bind
actually risks — occupying a port another service expects, or exposing a surface to a network the
operator did not intend — is answered by naming the endpoint rather than by classifying its range.
Exact matching rather than a pattern or a port range is `net.internal`'s choice for `net.internal`'s
reason: an operator writing a range hands back more than the endpoint they meant, and the grant's
value is that a reviewer can see what a deployment opened. An entry that does not parse as an endpoint
matches nothing.
