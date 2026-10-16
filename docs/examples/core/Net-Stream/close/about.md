Closes a TCP connection.

The other side can still read the bytes you wrote before `close`. After those bytes, its `read`
returns no bytes, which tells it that you closed the connection.

After `close`, `read` and `write` throw a `RuntimeError`, and so does a second `close`. Closing a
connection does not close the server that accepted it.

If your program does not call `close`, the connection is closed when the request ends. Call it
yourself when you are done, so the other side knows sooner. A `finally` block is a good place for
it, because it runs even when an error is thrown.
