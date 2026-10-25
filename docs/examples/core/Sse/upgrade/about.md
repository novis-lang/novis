Turns a request into an event stream that stays open after the request ends. The server can then
send the client new events at any time, for example each time something is published on a topic.

You name what the stream runs: the path of a file, or a static method written `Feed::run(...)`.
The stream starts when the request ends. It runs in its own isolate (a separate copy of the program
that shares no memory with the request). `args:` copies values into it. Inside the stream,
`Core\Sse::current()` returns the stream to send on.

The other options are the ones `spawn script` takes. `limits:` sets lower limits, such as
`{ memory: "8M" }`. `grants:` lists the capabilities the stream may use, and it gets only the ones
the request also has. `on:` can only be `"here"`, so `on: "worker"` does not compile.

A program run from the command line has no request, so the call throws a `RuntimeError`. A second
call in the same request also throws one.

**The examples below** open a stream that runs a file, show the error for a second call, and open a
stream of notifications only for a signed-in user.
