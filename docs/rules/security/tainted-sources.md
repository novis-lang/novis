Every member of the request, server, session, environment and CLI accessors, and a script's own
arguments, answers the `tainted` form of whatever it already returned. Header values and any other
client-influenced field are included; a fixed enum-shaped field such as the request method is not
attacker-shaped in the same way and is not required to be.

**An outbound reply's body is input in the same sense**, so a response body read back from a client
answers `tainted string`: pinning an address settles which host the bytes came from and says nothing
about what is in them, and a reply a program asked for is no safer than one it was sent. Its status
code is not tainted — three digits carry nothing a sink can misread. Values read back out of a
database are `tainted` under the same standing rule, which is what closes stored injection by the same
mechanism as reflected.

Structured input stays `array<mixed>`; the qualifier is about the scalar payload once it is pulled out
of `mixed`. The list of sources being **enumerable** is what lets the qualifier attach itself
automatically, and is exactly what `secret` has no equivalent of
(`rule:security/secret-has-no-ambient-source`).
