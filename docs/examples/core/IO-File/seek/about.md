Moves the handle of an open file to another byte.

`$file->seek($offset)` works on a `Core\IO\File` that `Core\IO::open` returned. The next `read`
or `write` then starts `$offset` bytes from the start of the file. `seek(0)` goes back to the
start. The offset always counts from the start of the file. To move forward from the current
position, use `seek($file->tell() + $n)`.

An offset past the end of the file is allowed. A `read` there returns an empty string. A
`write` there fills the gap before it with zero bytes.

A closed handle throws a `RuntimeError`. A handle that has no position, such as a pipe, throws
an `IOError`.
