Reads a part of an open file as bytes, exactly as they are stored.

`$file->readBytes($max)` works on a `Core\IO\File` that `Core\IO::open` returned. It reads up to
`$max` bytes from the current position, and moves the position past them. The next call starts
where this one stopped. When the file has no more bytes, the result is empty `bytes`.

Use it for a file that is not text, such as an image, an archive or a file in a binary format.
The bytes do not have to be valid UTF-8, and nothing is decoded or changed. To read text, use
`read`. It returns a `string` and never cuts a character in two.

A closed handle throws a `RuntimeError`. A handle that can only write throws an `IOError`.

**The examples below** check the start of an image file, read a file in fixed-size parts, and
read bytes that are not UTF-8.
