Three conditions must all hold for a protocol to belong in `Core`. **The failure mode is a library bug
rather than an application bug** — the mistake lives in the implementation of the protocol, not in how
the application uses it. **The failure is silent** — a wrong implementation returns a plausible result
rather than an error. **The need is near-universal** for the kind of program Novis exists to run.

One structural boundary decides the edge: **the operation is stateless over a key.** A protocol
requiring network round trips, a redirect dance, or stored per-flow state is a *flow*, not a token
operation. The major delegated-authentication and hardware-credential protocols are all flows: they
need clients, discovery documents, nonce storage and expiry policy, and their designs vary per
provider. They are out permanently, and are ordinary userland or package code built on this roster.

The closed list will be argued with, and this test is the answer — recording the boundary is what
makes it a decision rather than a negotiation.

**Not on disk.** Nothing enforces the test; it governs what a future member may be.
