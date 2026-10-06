Returns the size of a file in bytes.

`$info->size()` works on the `Core\IO\Metadata` value that `Core\IO::stat` returns. It returns a
`uint`: the number of bytes the file had when `stat` was called. An empty file has a size of `0`.

The size counts bytes, not characters. A character such as `ü` uses 2 bytes, so a text of 6
characters can be 8 bytes long.

The value does not change when the file changes later. To get the new size, call `stat` again.
The size of a folder is a number the operating system chooses, and it means different things on
different systems.
