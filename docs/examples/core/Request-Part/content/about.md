Returns the content of one uploaded file in pieces. A `foreach` loop over the result gives one
piece at a time, and each piece is `tainted bytes`. The program holds only the current piece, so a
large file never has to fit in memory. A file with no content gives no pieces.

Read the content while the `Core\Request::files()` loop is on this file. When the loop has moved to
the next file, the content of this one is gone, and reading it throws a `LogicError`. You can stop
early with `break`. The loop over the files then skips the rest of this file.

Use `readAll` when the file is small enough to hold as one value, and `saveTo` to write it to a file.

The examples show how to measure each file, how to stop at a size limit, and how to compute a
checksum while the file arrives.
