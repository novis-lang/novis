`Core\Server::traceId()` returns the id of the request that your program is answering. The id is
32 lower-case hexadecimal characters, such as `4bf92f3577b34da6a3ce929d0e0e4736`. Every request has
one, and the server writes the same id into its log lines.

A request can arrive with a `traceparent` header. This header comes from a service that called
yours, and it names a trace (a record of one task across several services). The id is then the one
in that header, so your logs and the caller's logs use the same id. Without the header, the server
creates a new random id for the request.

A command-line program answers no request, so `traceId` throws a `LogicError` there.

**Good to know:** show the id on an error page or put it in an error report. Then a person who
reports a problem can give you the id, and you can find the request in the logs.

**The examples below** print the id, show it on an error page, and catch the `LogicError` in a
command-line program.
