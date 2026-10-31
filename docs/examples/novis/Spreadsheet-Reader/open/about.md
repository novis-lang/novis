Opens a spreadsheet file so that your program can read it.

`$source` is the bytes of the file, or a `Core\IO\File` that your program opened for reading. The
bytes can come from an upload. The file can be xlsx, xlsm, xlsb, xls, ods or CSV, and
`Reader::open` finds the type from the bytes, not from a file name. Then you call `sheets` to list
the sheets and `rows` to read one of them.

A CSV file is UTF-8 text by default. `csv` sets its `separator`, `quote`, `escape` and `encoding`.
A file that starts with a UTF-16 byte order mark needs
no option. These options do not change how a workbook is read.

**Good to know:** a damaged file throws a `ParseError`. A CSV file has one sheet, `Sheet1`, and
each value in it is a string. A CSV file in another charset, an xls file and an ods file are read
whole when they are opened. Opening a file never runs its macros and never opens its links.

**The examples below** open an upload, read a CSV file, catch the error of a damaged file, and read
a CSV file with `;` in Windows-1252.
