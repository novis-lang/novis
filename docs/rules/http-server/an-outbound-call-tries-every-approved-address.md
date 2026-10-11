Every address a name resolves to is checked against `rule:security/net-address-policy`, and **one denied
address refuses the whole host**, naming it. A name that answers both a public address and one the policy
denies is what a rebinding attack looks like from the resolver's side, so the denied answer is not
quietly dropped from the set and the rest used; `net.internal`'s exceptions still apply per address. A
URL whose host is an IP address, and a `connectTo` value (`rule:http-server/an-outbound-call-names-its-address-only-under-a-grant`),
are a set of one.

`Core\Http\Target` carries **the approved set**, in the resolver's order and at most eight of it, and
still has no members, for `rule:http-server/allow-url-pins-the-address`'s reason: a program that could
read the set back out could rebuild a request around an address nothing approved. A retry reuses the set
and never re-resolves, so a retried call still performs exactly one resolution; a redirect hop resolves
and checks anew.

The connection falls back across the set RFC 8305's way — families interleaved from the resolver's first
answer, the next attempt started when the previous one has not connected within a fixed attempt delay,
the first to connect kept and the rest closed — all under the one `connectTimeout`, itself clamped by the
deadline (`rule:http-server/one-deadline-covers-the-whole-call`), so falling back introduces no new
bound. Every address failing is one `IOError` naming each, because an error naming only the last one
sends the reader to the wrong host. The lookup itself runs on the blocking pool per
`rule:http-server/a-core-is-never-blocked-on-a-syscall`, with the grant asked on the core before it and
the address check back on the core after it.

The attempt delay is `250ms`, RFC 8305's own recommendation, and a constant rather than a directive
because it trades against nothing an operator can see. Shorter spends a second socket on a set whose
first address was about to answer; longer leaves a call waiting on an unreachable address for most of
its budget.

- **Memory:** a `Target` holds at most eight addresses, charged to the request that laundered it. A call
  falling back holds at most one socket per address until one connects, and a lookup holds one job on
  the blocking pool under that pool's own bound.

**Declined:**
- **Keeping one pinned address and letting the connect fail.** One address is a single point of failure
  for a name that answers several, and it gains no security: the security is in checking every address,
  not in discarding all but one.
