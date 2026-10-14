Writes the spreadsheet to a file that your program already opened for writing.

`to` writes the file from the current position of `$file`, and returns a `Written` with the number
of rows of each sheet and the size in bytes. It does not close `$file`, so your program can write
more to it and then close it.

**Good to know:** an open file has no name to read the format from. The format is Excel (xlsx)
unless the export was made with the option `format: "csv"`.
