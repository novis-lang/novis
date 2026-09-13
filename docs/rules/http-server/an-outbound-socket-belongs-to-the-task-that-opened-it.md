An outbound socket is a value the program opened, so it belongs to the task that opened it: its
connection, its buffers and its reassembly space are charged to that task's budget, and when the task ends
the socket is closed with `1001` and its memory is released with everything else the task held. A request's
`wall_time` therefore bounds a socket opened inside a request without having to know what a socket is.

**It is never a root isolate.** A server-side connection is one because it *outlives* the request that
upgraded it (`rule:concurrency/a-connection-is-a-root-isolate`), and the arena, globals, budget and
timeline entry it is given are what that escape costs. An outbound socket has no request to escape from:
it is held by the running program that opened it, so an isolate of its own would put a boundary between
the program and the socket it is reading, costing an arena and a copy per socket and buying nothing. Memory
stays O(open sockets), and because no socket outlives its task, that is O(in-flight) rather than O(sockets
opened).

It never crosses an isolate boundary — `rule:classes/graph-copy` refuses it as it refuses every other
resource — so there is no question of who closes one. A program that wants a socket to outlive a request
opens it in something that outlives a request: a command, a queue job, a spawned script. Two `receive`s
waiting at once on one socket is a `LogicError`, because one message has one recipient and the alternative
is a fan-out policy invented for what is a program bug.
