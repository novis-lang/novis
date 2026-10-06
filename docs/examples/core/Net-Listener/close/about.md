Closes a TCP server. The server stops taking new clients, and its port is free
again for other programs.

Connections the server already accepted are not closed. Each one is a separate
`Core\Net\Stream`, so you can keep reading and writing it, and you close it on its own.

After `close`, `accept` and `port` throw a `RuntimeError`, and so does a second `close`.

If your program does not call `close`, the server is closed when the request ends. Call it yourself
when you are done, so the port is free sooner. A `finally` block is a good place for it, because it
runs even when an error is thrown.
