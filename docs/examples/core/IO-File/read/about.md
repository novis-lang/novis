Reads a part of an open file as text.

`$file->read($max)` works on a `Core\IO\File` that `Core\IO::open` returned. It reads up to
`$max` bytes from the current position, and moves the position past them. The next `read`
starts where this one stopped. When the file has no more bytes, `read` returns an empty string.

The result is always whole text. A character such as `é` uses more than one byte. If `$max`
ends inside a character, `read` returns the text before it. The next `read` starts with that
character.

A closed handle throws a `RuntimeError`. Bytes that are not UTF-8 text also throw a
`RuntimeError`, and the position does not move. A handle that can only write throws an
`IOError`.

This replaces PHP's `fread`.
