Reads the next line of an open file.

`$file->readLine()` works on a `Core\IO\File` that `Core\IO::open` returned. It reads from the
current position to the end of the line, and moves the position to the start of the next line.
The line break is not part of the result. `\n`, `\r\n` and `\r` all end a line, so a file from
any system is read the same way. A last line with no line break is still a line.

When the file has no more lines, `readLine` returns `null`. A loop that reads until `null` reads
the whole file, and only one line is in memory at a time. An empty line is an empty string, not
`null`.

A closed handle throws a `RuntimeError`. A line that is not UTF-8 text also throws a
`RuntimeError`, and the position does not move. A handle that can only write throws an
`IOError`.

This replaces PHP's `fgets`.
