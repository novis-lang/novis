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
different typed members beside each other. They disagree about the body's type and its content type,
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
