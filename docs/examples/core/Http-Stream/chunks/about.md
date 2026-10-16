Reads the body of a streamed reply piece by piece, exactly as the bytes arrive.

Each piece is `tainted bytes`, because another server sent it. The pieces are not split at lines or
at any other boundary. How many pieces there are, and how large each one is, depends on the network.
Your program only knows that the pieces together are the whole body, in order.

The body of a stream can be read only once. After `chunks()`, a call to `chunks()`, `lines()`,
`events()` or `saveTo()` on the same stream throws a `LogicError`.

**Good to know:** you can stop the loop early with `break`. The rest of the body is then not read.
Use `saveTo()` to write the body to a file.

**The examples below** count the size of a download, show that the body can be read only once, and
stop a download that is larger than a limit.
