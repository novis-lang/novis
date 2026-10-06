Returns where the handle of an open file is.

`$file->tell()` works on a `Core\IO\File` that `Core\IO::open` returned. It returns the number
of bytes from the start of the file to the handle. This is where the next `read` or `write`
starts. A new handle is at `0`, unless it was opened with `Core\IO\FileMode::Append`.

`read`, `readLine` and `write` move the handle past the bytes they used. `tell` counts bytes,
not characters, so a character such as `é` counts as 2. The number `tell` returns can be given
to `seek` later to come back to the same place.

A closed handle throws a `RuntimeError`. A handle that has no position, such as a pipe, throws
an `IOError`.
