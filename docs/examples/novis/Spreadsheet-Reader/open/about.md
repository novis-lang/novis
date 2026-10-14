Opens a spreadsheet file so that your program can read it.

`$source` is the bytes of the file, or a `Core\IO\File` that your program opened for reading. The
bytes can come from an upload. The file can be xlsx, xlsm, xlsb, xls, ods or CSV, and
`Reader::open` finds the type from the bytes, not from a file name. Then you call `sheets` to list
the sheets and `rows` to read one of them.

**Good to know:** a damaged file throws a `ParseError`. Catch it to tell the person what is wrong
with the upload. A CSV file is UTF-8 text with one sheet, `Sheet1`, and each value in it is a
string. An xls or ods file is read whole when it is opened. Opening a file never runs its macros and
never opens its links.

**The examples below** open an upload, read a CSV file, and catch the error of a damaged file.
