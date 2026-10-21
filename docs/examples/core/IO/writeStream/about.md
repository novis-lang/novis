Writes a file from many small pieces of data, one piece at a time.

`Core\IO::writeStream` takes a path and a list of `bytes` pieces, called chunks. It writes them in
order. The list can be an `array<bytes>` or an iterator that makes each chunk when it is needed.
Only one chunk is in memory at a time, so a large upload needs little memory.

The option `max` is the largest number of bytes the file may have. Bigger data throws a
`RuntimeError`. The option `overwrite` is `false` by default. Then an existing file throws an
`IOError` and is not changed.

When a write fails part of the way through, `writeStream` deletes the file before it throws. No
half-written file stays on the disk.

A command-line program may write any path. On a server, a program may write only in the folders
from `Core\IO::temporaryDir`. Any other path needs the `fs.write` capability. Without it,
`writeStream` throws a `RuntimeError` and creates no file.
