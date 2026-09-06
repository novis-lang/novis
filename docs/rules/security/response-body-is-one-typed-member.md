A response body is written by one of five typed members, each owning a body shape and setting its own
content type. The HTML member takes the carrier and so has nothing to refuse; the JSON and text
members are contagious; the bytes member is contagious in its body and a sink in its content type; the
file member's path is a sink.

The JSON member accepts a tainted value freely, because the framing belongs to the serializer and
never to concatenation — a tainted string becomes a JSON string value and cannot escape it. The text
member accepts one only because a no-sniff header is on by default with nothing configured, so
`text/plain` is not re-parsed as HTML; that dependency is stated so removing the default is visibly a
change to two rules.

**`echo` and a typed writer on the same response is a compile error.** They disagree about the body's
type and its content type, and silently letting the last one win is how a JSON endpoint acquires an
HTML prelude.
