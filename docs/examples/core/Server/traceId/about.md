`Core\Server::traceId()` returns the id of the request that your program is answering. The id is
32 lower-case hexadecimal characters, such as `4bf92f3577b34da6a3ce929d0e0e4736`. The server writes
the same id into its log lines.

Every response has a `traceresponse` header, such as
`00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00`. The second part is the id. To remove the
header, set `response_header = false` in the `[trace]` block of `nvs.toml`. The id and the log lines
do not change.

A request can have a `traceparent` header from a service that called yours. The id is then the one
in that header, so both services log the same id. Otherwise the server creates a random id. The
server ignores an `X-Request-ID` header.

A command-line program answers no request, so `traceId` throws a `LogicError` there.

**Good to know:** show the id on an error page. A person who reports a problem can give you the id,
and you can find the request in the logs.
