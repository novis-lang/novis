`net.connect` is not a boolean and not merely a host list. It carries an address policy enforced on
**every outbound connection whose address the program supplies**, hardcoded URLs included, because a
hardcoded hostname can resolve into a private range and because deployment configuration supplies most
real endpoint URLs.

Denied by default: loopback, the private ranges, **link-local**, unspecified, and the IPv4-mapped IPv6
forms of all of them. An operator grants an exception as an **IP address literal** beside the connect
grant. Three things it is not, each a widening this refuses: not a hostname, because the policy is
asked of a resolved address and a name would except whatever it resolved to afterwards; not a range,
because an operator writing a whole `/8` hands back most of the table without naming a host; and not
`true`, which is the one place a capability's `true` does not mean everything.

The one class of address it does not govern is an endpoint an operator wrote into root-owned
configuration and granted by name — that address is not attacker-influenceable, and applying the
policy there would deny every ordinary deployment. A program-supplied target stays governed in full.
