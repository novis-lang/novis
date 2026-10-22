`net.connect` is not a boolean and not merely a host list. It carries an address policy enforced on
**every outbound connection whose address the program supplies**, hardcoded URLs included, because a
hardcoded hostname can resolve into a private range and because deployment configuration supplies most
real endpoint URLs.

Denied by default: every address that is not globally routable. That is loopback, the private ranges,
**link-local**, unspecified, shared address space (`100.64.0.0/10`), protocol assignments
(`192.0.0.0/24`), benchmarking (`198.18.0.0/15`), multicast (`224.0.0.0/4`, `ff00::/8`), reserved
(`240.0.0.0/4`), broadcast, IPv6 site-local (`fec0::/10`) and the local-use NAT64 prefix
(`64:ff9b:1::/48`). That prefix is denied whole, because RFC 6052 lets its translator carry an IPv4
address in any of six layouts and the layout is the translator's setting, so no reading of an address
says which IPv4 address it reaches; an operator who runs one writes the prefix in `net.internal`. It is also every IPv6 form that carries one of those IPv4 addresses,
each of which reaches the address it carries: IPv4-mapped (`::ffff:0:0/96`), IPv4-compatible
(`::/96`), the NAT64 well-known prefix (`64:ff9b::/96`), 6to4 (`2002::/16`, the 32 bits after the
prefix) and Teredo (`2001::/32`, the inverted low 32 bits). The documentation ranges (`192.0.2.0/24`,
`198.51.100.0/24`, `203.0.113.0/24`, `2001:db8::/32`) are not denied: nothing routes them, so a
connection to one fails on its own. An operator grants an exception in `net.internal` as an **IP
address or a CIDR range written out** (`10.0.0.5`, `172.18.0.0/16`): a container network or a subnet
the operator runs is a range, and no list of addresses can follow it. Two things it is not, each a
widening this refuses: not a hostname, because the policy is asked of a resolved address and a name
would except whatever it resolved to afterwards; and not `true`, which is the one place a capability's
`true` does not mean everything. A range with bits set below its prefix (`10.0.0.5/8`) reads as one
host and would grant a block, so it is refused at load with the hostname and `true`.

**The application's own code reaches every public host by default**, on a CLI run and in server work,
so an unwritten `connect` is every host and the table above is the whole policy. A written `connect`
list narrows that to an allowlist. **Loopback (`127.0.0.0/8`, `::1`) is excepted by default on a CLI
run and in server work whose configuration names `development`.** That default is a startup row
(`rule:config/a-startup-default-is-never-flipped`), so a program that flips itself to `development`
gains nothing, and a production server reaches loopback only where `internal` names it. A written
`internal` list replaces the default whole.

Link-local is the range whose omission is catastrophic rather than regrettable: `169.254.169.254` is the
cloud instance-metadata endpoint, and reaching it turns a feature that fetches a URL into credential
theft. The exception and the connect grant are independent and both required — `net.internal` reaches
nothing a written `net.connect` list does not name — so an exception is a second thing an operator
writes, not a way around the first. A production deployment that calls internal services has to say
so, and that is the point: the difference between an intended internal call and a forged one is
whether somebody wrote it down.

The one class of address it does not govern is an endpoint an operator wrote into root-owned
configuration and granted by name — that address is not attacker-influenceable, and applying the
policy there would deny every ordinary deployment. A program-supplied target stays governed in full.

A configured forward proxy is such an endpoint and is not asked the table's question; and under
`[http.client.proxy] resolve = "proxy"` the destination's resolved address is not asked it either,
because Novis never sees one — `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`
is where the operator writes that word and what the boot says every time they have.

"Connection" here means every outbound destination, not only a connected stream: a `Core\Net` datagram
sent to an address the program supplied is asked the same question at the send that a TCP connect is
asked at the connect. What the table does **not** govern is a *bind*, whose terms invert —
`rule:security/net-listen-is-a-separate-grant-from-net-connect` is that grant and says why it carries
no policy of its own.

**Declined:**
- **Reading `64:ff9b:1::/48` as the IPv4 address in its low 32 bits**, as the well-known prefix is
  read. A translator set to a `/64` layout puts the address elsewhere, so a private address written
  that way reads as a public one. Denying the prefix costs an IPv6-only network that uses it one
  `net.internal` line.
- **No exception syntax at all**, so a private range is never reachable from a program-supplied
  address. Service-to-service calls are the normal case in any non-trivial deployment, and a rule that
  has to be worked around is worse than one that has to be written down.
- **The policy applied to an operator-written endpoint too.** A database or a shared store lives at a
  private address, a container network or loopback, so every ordinary deployment would be denied
  (`rule:core-classes/db-capabilities`, `rule:config/cache-shared-is-the-grant-over-the-configured-store`).
- **Loopback always allowed, a production server included.** It would give up SSRF protection for
  every service listening on the machine, such as an admin port or an unauthenticated store, to save a
  production deployment one line.
