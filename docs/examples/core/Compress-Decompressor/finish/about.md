Closes a decompression stream and returns everything you added to it, decompressed.

You call `finish` once, after your last `add`. It joins your pieces in the order you added them,
reads them as one block of compressed data, and returns the result as `tainted bytes` — data that
came from outside your program, which Novis tracks so you cannot use it somewhere unsafe by
accident.

This is where the data is checked. If the pieces are not a valid block of the format you chose, or
if the result would be larger than the size limit the stream was opened with, `finish` throws a
`ParseError`. It never returns a part of the result.

**Good to know:** the stream ends here, whether `finish` returned or threw. Both `add` and `finish`
throw a `RuntimeError` on it from now on.
