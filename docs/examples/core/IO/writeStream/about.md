Writes a file from many small pieces of data, one piece at a time.

`Core\IO::writeStream` takes a path and a list of `bytes` pieces, called chunks. It writes the
chunks to the file in order. The list can be an `array<bytes>` or an iterator that makes each
chunk when it is needed. Only one chunk is in memory at a time, so a large upload or download can
be saved without holding the whole file in memory.

Two options change what `writeStream` does. `max` is the largest number of bytes the file may
have. When the chunks are bigger than that, `writeStream` throws a `RuntimeError`. `overwrite`
says if an existing file may be replaced. It is `false` by default, so a file that is already
there throws an `IOError` and stays as it was.

When a write fails part of the way through, `writeStream` deletes the file before it throws. So
the program never finds a file that is only half written.

The program needs the `fs.write` capability for the path. Without it, `writeStream` throws a
`RuntimeError` before any file is created.
