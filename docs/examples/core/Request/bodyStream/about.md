Returns the body of the request in pieces, in the order they arrive. A `foreach` loop reads the
pieces one at a time, and each piece is a `tainted bytes` value.

Use it when the body can be large, for example an uploaded backup or video. Your program handles
each piece and can then forget it, so the whole body is never in memory at once. How the body is
split into pieces has no meaning, and a word or a character can be split between two pieces. When
the request has no body, the loop runs zero times.

The body can be read only once in this way. After `bodyStream`, the methods `body`, `bytes`,
`json` and `files` throw a `LogicError`. Calling `bodyStream` after one of them throws a
`LogicError` too. A command-line program answers no request, so `bodyStream` throws a
`LogicError` there.
