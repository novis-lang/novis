Returns one header of a reply as a string, or `null` when the reply does not have that header.

The name does not depend on upper or lower case, so `content-type` and `Content-Type` give the same
result. When a server sends the same header on more than one line, `header()` joins the lines with
`, ` in the order they arrived. The value comes from another server, so it is `tainted` until your
program checks it.

`Set-Cookie` is the one header that `header()` does not read. Two cookies joined with a comma cannot
be read correctly, so the call throws a `LogicError`. Use `headers()` for cookies.

**Good to know:** the examples use `Core\Test::answerHttp` to give fixed replies, so they run
without a network. They read a content type, show a header sent twice, and read `Retry-After` from
a service that is busy.
