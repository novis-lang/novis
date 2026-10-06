When a design trades memory footprint against security, latency or simplicity, Novis pays the memory.
The ordering, highest first; a lower item is spent to buy a higher one, never the reverse:

1. **Security and request isolation** — not traded for anything, including for the three below.
2. **Latency and throughput on the request path.**
3. **Simplicity** — of the language surface first, then of the implementation.
4. **Memory footprint** — last, and spent deliberately to buy any of the above.

**A priority is cited by its name, never by its number**, so a citation survives any change to the
list. What a program observably does is not in the ordering: the rules state it and the conformance
cases pin it.

**Novis is not a low-footprint runtime.** It does not target `no_std`, microcontrollers or minimum-RSS
deployments, and there is no cut-down build trading behaviour for size. "Uses less memory" is not on
its own an argument for a change; "uses less memory and is no more complicated" is simply a better
design and needs no appeal to any of this.

Three bounds, and none of them is licence to leak. **Bounded, not merely modest** — every per-request
allocation stays under a cap the runtime can *enforce*, and consumption that cannot be attributed to a
request and capped is not a trade-off but a hole in isolation. **O(in-flight), not O(served)** — cost
is proportional to concurrent work and released wholesale when that work ends; growth proportional to
requests served is a leak, and no ordering makes a leak acceptable. **Footprint, not traffic** — bytes
held are cheap and bytes moved are not, so an extra allocation or cache miss on a hot path is a
latency question, however much it looks like a memory one.

**Say what you spend.** A decision that takes memory states the amount, per request or per task, in
the doc comment or record that carries it. The ordering is only usable by a later reader if the costs
are written down as they are incurred.
