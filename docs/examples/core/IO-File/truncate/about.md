Sets the length of an open file.

`$file->truncate($size)` works on a `Core\IO\File` that `Core\IO::open` returned. After it, the
file is `$size` bytes long. A smaller size deletes every byte after `$size`. A larger size adds
zero bytes at the end. `truncate(0)` empties the file.

`truncate` does not move the handle. If the handle was after the new end, it stays there, and
the next `write` leaves a gap of zero bytes before it. To write from the start of an emptied
file, call `seek(0)` after `truncate(0)`.

A closed handle throws a `RuntimeError`. A handle opened with `Core\IO\FileMode::Read` throws
an `IOError`, because the file cannot be changed through it.
