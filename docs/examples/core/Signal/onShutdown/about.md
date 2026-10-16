`Core\Signal::onShutdown` registers a function that runs when the process is asked to stop. This
happens when an operator stops the server, or when the program gets a stop signal such as Ctrl-C or
`SIGTERM`. All of these stop signals have the same effect.

The function runs once, between two statements of your program. At that time the server already
accepts no new requests, and `Core\Server::isDraining()` returns `true`. The function gets no
arguments. It cannot stop the shutdown or delay it. If it throws an error, the rest of it does not
run. After it returns, your program continues and should finish its work quickly.

Each request has its own function. A second call replaces the first one, and the function is
deleted when the request ends. If nothing asks the process to stop, the function never runs.

**The examples below** register a message, replace one function with another, and let a worker
stop between two jobs.
