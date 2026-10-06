Reads the bytes that have arrived on a TCP connection.

`read` returns at most `$max` bytes. It often returns fewer, because it gives you what has arrived
so far and does not wait for more. A program that needs a whole message calls `read` again until it
has all of it.

When the other side has closed the connection and every byte was read, `read` returns an empty
`bytes` value. If nothing arrives within `$within`, it throws a `TimeoutError`, and the connection
still works afterwards.

**Good to know:** the result is tainted, because another computer sent it. Check it before you use
it in a query, a command or a page.

**The examples below** show a read that is shorter than the data, a read that times out, and a loop
that reads a whole reply until the other side closes.
