`Core\Server::traceId()` returns the id of the request that your program is answering. The id is
32 lower-case hexadecimal characters, such as `4bf92f3577b34da6a3ce929d0e0e4736`. Every request has
one, and the server writes the same id into its log lines.

The server also sends the id back in the `traceresponse` header of every response, in the form
`00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00`. The second part is the id. The server
ignores an `X-Request-ID` header that a client sends.

To stop sending the header, set `response_header = false` in the `[trace]` block of `nvs.toml`.
The request still has its id, and the log lines still contain it.

A request can arrive with a `traceparent` header from a service that called yours. The id is then
the one in that header, so your logs and the caller's logs use the same id. Without the header, the
server creates a new random id.

A command-line program answers no request, so `traceId` throws a `LogicError` there.

**Good to know:** show the id on an error page. A person who reports a problem can then give you
the id, and you can find the request in the logs.

**The examples below** print the id, show it on an error page, and catch the `LogicError` in a
command-line program.
