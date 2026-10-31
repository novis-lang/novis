Opens a spreadsheet file so that your program can read it.

`$source` is the bytes of the file, or a `Core\IO\File` that your program opened for reading. The
file can be xlsx, xlsm, xlsb, xls, ods or CSV. `Reader::open` finds the type from the bytes, not
from a file name. Then `sheets` lists the sheets, and `rows` reads one of them.

A CSV file is UTF-8 text by default. `csv` sets its `separator`, `quote`, `escape` and `encoding`.
A file that starts with a UTF-16 byte order mark needs no option.

**Good to know:** a damaged file throws a `ParseError`. Bytes that are not a spreadsheet also throw
a `ParseError`, such as an image or a file with a NUL byte. A CSV file has one sheet, `Sheet1`, and
each value in it is a string. Opening a file never runs its macros and never opens its links. If a
workbook file changes on disk after `Reader::open`, the next call throws a `LogicError`.

**The examples below** open an upload, read a CSV file, catch the error of a damaged file, and read
a CSV file with `;` in Windows-1252.
