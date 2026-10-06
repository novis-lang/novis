Writes text to an open file.

`$file->write($data)` works on a `Core\IO\File` that `Core\IO::open` returned. It writes `$data`
at the current position, and moves the position past it. The next `write` continues where this
one stopped, so a program can write a large file in many small parts.

`write` returns the number of bytes it wrote. This is always the whole of `$data`, so the
program never has to write the rest itself. A character such as `é` uses more than one byte, so
the number can be larger than the number of characters. An empty string writes nothing and
returns `0`.

A closed handle throws a `RuntimeError`. A handle that can only read throws an `IOError`, and so
does a write that fails, for example on a full disk.
