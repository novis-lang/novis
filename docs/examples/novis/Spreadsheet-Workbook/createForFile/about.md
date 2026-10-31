Starts a spreadsheet that your program writes to a file at a path.

The file is created, or replaced if it already exists. A path that ends in `.csv` makes a CSV file,
and any other path makes an Excel file (xlsx). Each part of the file is written as soon as it is
ready, so a large export does not have to fit in memory. `finish` writes the end of the file and
closes it. The `Written` it returns has the number of rows of each sheet and the size of the file.
Its `bytes` is `null`, because the file is on the disk.

**Good to know:** a program you start from the command line may write any path. On a server, a
program needs the `fs.write` capability in `nvs.toml` for the path. Without it, `createForFile`
throws an error and writes nothing.

A CSV file is UTF-8 with `,` between values. `csv` sets its `separator`, `quote` and `encoding`.

**The examples below** save an Excel file and read it back, save a CSV file, save two sheets with
the summary as the first tab, and save a CSV file with `;` in Windows-1252.
