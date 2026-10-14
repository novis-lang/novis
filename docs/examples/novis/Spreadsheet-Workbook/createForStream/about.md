Starts a spreadsheet that your program writes to a file it has already opened.

Open the file with `Core\IO::open` for writing, and pass it to `createForStream`. Each part of the
spreadsheet is written to the file as soon as it is ready. An open file has no name, so the file is
an Excel file (xlsx) unless you set `format: "csv"`.

`finish` writes the end of the spreadsheet and does not close the file. Your program can write more
to the file after that, and it closes the file when it is done.

**The examples below** write a CSV file, write two exports into one file, and write an Excel file and
read it back.
