A response body is written by one of seven typed members, each owning a body shape and setting its own
content type. The HTML member takes the carrier and so has nothing to refuse; the JSON and text
members are contagious; the bytes member is contagious in its body and a sink in its content type; the
file member's path is a sink.

**Two of the seven write their body over time rather than at once, and classification follows the
shape and not the timing.** The streaming member declares a media type it is told, so that argument
is the bytes member's sink for the bytes member's reason — it becomes an instruction the peer obeys
about how to read everything after it — while each chunk is a union of text and bytes, which carries
no classification at all and therefore refuses a tainted argument outright. That is the fail-closed
direction of the two: the text member accepts a tainted body and a chunk does not.

The event-stream member takes no media type — the protocol's is the only one it could have — and its
`send` splits three ways. The payload is contagious, for the JSON member's reason: framing belongs to
us and to the serializer, and normalization happens before the payload is split across `data:` lines,
so it cannot reach any other line. The event name and the id are **sinks**: a client dispatches on
the name and echoes the id back in its next request's `Last-Event-ID`, so an attacker-chosen one is
the cross-tenant hazard a tainted topic name is.

The JSON member accepts a tainted value freely, because the framing belongs to the serializer and
never to concatenation — a tainted string becomes a JSON string value and cannot escape it. The text
member accepts one only because a no-sniff header is on by default with nothing configured, so
`text/plain` is not re-parsed as HTML; that dependency is stated so removing the default is visibly a
change to two rules.

**Two writers of one response body is a compile error** — `echo` beside a typed member, and two
different typed members beside each other. A file's text outside `<?nvs … ?>` is `echo`'s writer
here, because the HTML sink writes it the same way. They disagree about the body's type and its content type,
and silently letting the last one win is how a JSON endpoint acquires an HTML prelude. What the
refusal needs is two *different* writers: one member called twice declares one thing twice, so a body
written in a loop is an ordinary program and is left alone.

**That error is reported where a response is statically certain, which is a `#[Route]` handler.**
`echo` is bound by context and not by syntax — the same body writes a response under a request and a
terminal sink under `nvs run` — so a method that is not a handler writes no response for two writers
to disagree over and is left alone rather than refused on suspicion. One response is outside the
check and stays there: a mount's entry script, whose top-level frame is the request body and which is
the same compiled unit the command line runs, so nothing distinguishes the two uses at compile time.
There the default decides instead of the refusal — `echo` alone means `text/html`, and a typed member
written beside it wins the content type it declared last.

**The file member, `sendFile(string $path)`, takes the path alone.** A download name is a
`Content-Disposition` header, which `setHeader` already admits, and what the file's bytes are called
is the static-file policy's media-type table, which also owns its `Range` and conditional handling.

- **Memory:** a file body costs the request one path, and the connection one open handle and one
  range's read buffer while the response is in flight. The file is never copied through the request.

**Declined:**
- **A filename parameter on the file member.** A download name is a header, and `setHeader` writes it.
- **A content-type parameter on the file member.** It would let a response declare one type while
  the static-file policy's range and conditional handling served the same path as another.
- **Keying the body's rule on its `Content-Type` rather than on the member that wrote it.** The header
  can be set after the first byte of the body is written, so the rule would depend on a fact not yet
  decided where it must be enforced.
- **One `Response::write` that takes `Core\Html\Markup` for every body.** One rule instead of several
  members, but it makes `rule:security/assert-trusted` the ordinary way to write an API handler, which
  trains the escape hatch on the most common handler there is.
