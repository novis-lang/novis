Turns a request into an event stream that stays open after the request ends. The server can then
send the client new events at any time, for example each time something is published on a topic.

You call `upgrade` in the request, and you name what the stream runs: the path of a file, or a
static method written `Feed::run(...)`. The stream starts when the request ends. It runs in its own
isolate (a separate copy of the program that shares no memory with the request). `args:` copies
values into it. For a method, each value goes to the parameter with the same name. Inside the
stream, `Core\Sse::current()` returns the stream to send on.

Any request can be upgraded. A program run from the command line has no request, so the call throws
a `RuntimeError`. A second call in the same request also throws a `RuntimeError`.

**The examples below** open a stream that runs a file, show the error for a second call, and open a
stream of notifications only for a signed-in user.
