Refused on a request Novis receives **and on a response Novis's outbound client receives**:

| Defect | Why it is ambiguous |
|---|---|
| an `obs-fold` continuation line | two readers differ on where the value ends |
| a header line with no colon | it is not a header, and treating it as one invents a name |
| CTL, NUL, CR or LF in a header name or value | the framing itself becomes reader-dependent |
| duplicate `Content-Length` | two answers to how long the body is |
| `Content-Length` with `Transfer-Encoding` | the classic smuggling pair |
| NUL anywhere in a hostname | the name resolved is not the name written |
| a `Location`, hostname or URI over its configured maximum | truncating produces a *different, valid* value, which is worse than an error |
| a request path with a dot-segment or an encoded separator | the path a mount is selected by and the path a reader sees come apart |
| an `X-Forwarded-For` token in the trusted-walk position that neither names an address nor withholds one | it was about to be used as the client's address and cannot be read as one |

A token that *withholds* an address — `unknown` — has said something definite and is not refused.

A folded header line is refused rather than replaced, the stricter of the two answers the HTTP
specification allows a server, so a client that still sends one cannot talk to Novis.

`Content-Length` with `Transfer-Encoding` on an inbound request is the one row the server answers
differently: it follows the HTTP/1.1 specification, the chunked encoding wins and the length is
ignored. The connection is closed after that message, so bytes a proxy read as a second request are
never read as one here.

**There is no opt-out on the outbound half.** A remote server is an attacker too, and a response's
body flows straight into program logic. An upstream emitting genuinely ambiguous headers is one
whose answers cannot be reasoned about, so a throw at the call site is the honest outcome. The cost is
that an upstream with a sloppy header emitter breaks an outbound call, visibly, at the call site.

**Declined:**

- **A per-call opt-out on the outbound half.** It would buy integration with sloppy third-party
  services, which is a real problem. The second code path would have to stay exactly as safe as the
  first, and an option like that is copy-pasted into the calls that did not need it.
