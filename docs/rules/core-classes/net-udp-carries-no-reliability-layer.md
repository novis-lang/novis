A `Core\Net` datagram socket sends and receives messages, reports what it sent and what arrived, and
offers nothing above that: no ordering, no retransmission, no fragmentation and no acknowledgement. A
datagram that is lost is lost, and a program that cannot tolerate that wants TCP.

The refusal is written down because the pressure to add "just a retry" is constant and its result is
always the same — a reliability layer nobody specified, whose failure modes belong to the library
while the timing budget it spends belongs to the program. A protocol that genuinely provides reliable
datagrams is a protocol, placed by `rule:core-api/tier-placement` like any other, and Tier 0 is not
where it lands.

A program that builds ordering or retries over this surface is making that choice explicitly, which is
the point: the trade is visible in the program rather than hidden in a member's contract.
