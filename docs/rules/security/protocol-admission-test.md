Three conditions must all hold for a protocol to belong in `Core`. **The failure mode is a library bug
rather than an application bug** — the mistake lives in the implementation of the protocol, not in how
the application uses it. **The failure is silent** — a wrong implementation returns a plausible result
rather than an error. **The need is near-universal** for the kind of program Novis exists to run.

One structural boundary decides the edge: **the operation is stateless over a key.** A protocol
requiring a redirect dance, discovery documents, or per-flow state that `Core` would have to store is a
*flow*, not a token operation. The delegated-authentication protocols are all flows: they need clients,
discovery documents, nonce storage and expiry policy, and their designs vary per provider. They are out
permanently, and are ordinary userland or package code built on this roster.

WebAuthn's server half is inside the line: each verification is a signature check over a public key
the application stores, against a challenge the started session keeps. Its attestation part, which
needs vendor certificate lists, stays out. Sending one encrypted push message is inside it too: one
request out, with nothing stored and nothing to come back for.

The closed list will be argued with, and this test is the answer — recording the boundary is what
makes it a decision rather than a negotiation.

The test is applied in review, to every member proposed for the roster; no check in the tree runs
it.
