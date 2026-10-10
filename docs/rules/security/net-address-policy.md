`net.connect` is not a boolean and not merely a host list. It carries an address policy enforced on
**every outbound connection whose address the program supplies**, hardcoded URLs included, because a
hardcoded hostname can resolve into a private range and because deployment configuration supplies most
real endpoint URLs.

Denied by default: loopback, the private ranges, **link-local**, unspecified, and every IPv6 form that
carries one of them in its low 32 bits — IPv4-mapped (`::ffff:0:0/96`), IPv4-compatible (`::/96`) and
the NAT64 well-known prefix (`64:ff9b::/96`), each of which reaches the address it carries. An operator grants an exception as an **IP address written out** beside the connect
grant. Three things it is not, each a widening this refuses: not a hostname, because the policy is
asked of a resolved address and a name would except whatever it resolved to afterwards; not a range,
because an operator writing a whole `/8` hands back most of the table without naming a host; and not
`true`, which is the one place a capability's `true` does not mean everything.

Link-local is the range whose omission is catastrophic rather than regrettable: `169.254.169.254` is the
cloud instance-metadata endpoint, and reaching it turns a feature that fetches a URL into credential
theft. The exception and the connect grant are independent and both required — `net.internal` reaches
nothing `net.connect` has not granted by name — so an exception is a second thing an operator writes, not
a way around the first. A deployment that calls internal services has to say so, and that is the point:
the difference between an intended internal call and a forged one is whether somebody wrote it down.

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
- **No exception syntax at all**, so a private range is never reachable from a program-supplied
  address. Service-to-service calls are the normal case in any non-trivial deployment, and a rule that
  has to be worked around is worse than one that has to be written down.
- **The policy applied to an operator-written endpoint too.** A database or a shared store lives at a
  private address, a container network or loopback, so every ordinary deployment would be denied
  (`rule:core-classes/db-capabilities`, `rule:config/cache-shared-is-the-grant-over-the-configured-store`).
