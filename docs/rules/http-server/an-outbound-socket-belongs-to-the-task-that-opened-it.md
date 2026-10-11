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

**`close` sends its frame and waits for the peer's under the send wait.** A peer that never answers
is closed anyway, and that is not an error: the connection is gone either way, and a throw would put a
`catch` around every normal ending. This end does not reconnect on its own either
(`rule:http-server/an-outbound-socket-has-no-compression-and-no-reconnect`).

- **Memory:** an open socket holds one connection, a TLS session for `wss`, `tungstenite`'s read and
  write buffers, and at most `maxMessage` for a message being reassembled, all charged to the task that
  opened it and released when it closes. Nothing is held per core or per process.

**Declined:**
- **A socket handed to another isolate.** Who closes it and whose budget holds it would become runtime
  questions, for a pattern a program writes by opening the socket where it reads it.
