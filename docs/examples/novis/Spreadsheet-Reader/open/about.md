Opens a spreadsheet file so that your program can read it.

`$source` is the bytes of the file, or a `Core\IO\File` that your program opened for reading. The
file can be xlsx, xlsm, xlsb, xls, ods or CSV. `Reader::open` finds the type from the bytes. Then
`sheets` lists the sheets, and `rows` reads one of them.

A CSV file is UTF-8 text by default. `csv` sets its `separator`, `quote`, `escape` and `charset`.
A first line such as `sep=;` sets the separator and is not a row, unless you set `separator`.
A CSV file has one sheet, `Sheet1`, and each value in it is a string.

**Good to know:** a damaged file throws a `ParseError`. So does a file that is not a spreadsheet,
and a CSV file that is not UTF-8. A file saved with a password throws a `RuntimeError`. Opening a
file never runs its macros and never opens its links. If the file changes on disk after
`Reader::open`, the next call throws a `LogicError`.

**The examples below** open an upload, read a CSV file, catch the error of a damaged file, and read
a CSV file with `;` in Windows-1252.
