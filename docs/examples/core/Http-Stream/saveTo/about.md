Writes the body of a streamed reply straight to a file, as the bytes arrive.

The program never holds the whole body in memory, so it can save a download that is larger than
its memory. The second value is the largest size you accept, in bytes. It is required, because the
other server decides how large the reply is.

`saveTo()` throws a `RuntimeError` when the body is larger than that limit. It throws an `IOError`
when a file already exists at the path, so an existing file is never replaced. After a failed save,
the part already written is deleted. The program needs the `fs.write` capability for the path.

The body of a stream can be read only once. After `saveTo()`, a call to `chunks()`, `lines()`,
`events()` or `saveTo()` on the same stream throws a `LogicError`.

**The examples below** save a report, stop a download that is too large, and run a backup job that
never replaces a file.
