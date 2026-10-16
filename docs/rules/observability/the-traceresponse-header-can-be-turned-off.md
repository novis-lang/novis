`[trace] response_header` decides whether a response carries the `traceresponse` header, is `true` by
default in both modes, and `false` removes the header and nothing else. The trace id is still drawn
for every request, and the log records, the error renderings, the access line and the spans still
carry it (`rule:observability/a-trace-id-exists-for-every-request`).

On by default because the header is how a proxy, a browser's network panel and the debug viewer
(`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`) move from one response to
everything recorded about it, and a default that hid it would cost every developer that path. A switch
because a deployment may count the id as a correlation handle it does not want to hand to clients: two
responses with ids from the same trace show they belong together, and an id an operator searches logs
by is one more value an attacker can quote back. The operator who weighs that decides it, in the file.

The key sits in `[trace]` beside `propagate`, the other key that decides whether the id leaves the
process, and takes that block's class: `System`, read per response off the snapshot the request ran
under, so a reload reaches the next response. A program cannot turn it on or off for itself. It
governs every answer the door writes, a refusal at the valve included, at the one point that writes
the header (`crates/nvs-server/src/serve.rs`'s `counted`). With the header off, a `traceresponse` the
program set for itself is still removed, because the request has one id and it is the door's.

- **Memory:** nothing. With the header off a response is 55 bytes shorter.

**Declined:**
- **Sending the header in development only.** A proxy or a client that logs it in production is the
  case the header exists for, and a mode default would make the most common deployment lose it silently.
- **Letting a program's own `traceresponse` through when the header is off.** The request would then
  answer with an id that is not its trace id.
