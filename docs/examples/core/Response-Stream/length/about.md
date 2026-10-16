Returns how many bytes your program writes to the stream. It works on a stream that
`Core\Response::stream` started with a `$length`.

`length()` is the size of the whole body, or the size of the part the browser asked for with a
`Range` header. It is `0` when nothing is sent. That happens when the browser already has this
version of the body, or when it asked for a part the body does not have.

Your writes must add up to exactly this number. A write past it throws a `LogicError` and sends
nothing. If the program ends before it has written this many bytes, the connection is closed. The
browser then knows that the body is not complete.

A stream without a `$length` throws a `LogicError` here.

The examples show a part written in small pieces, a body with a version name, and a write past the
length.
