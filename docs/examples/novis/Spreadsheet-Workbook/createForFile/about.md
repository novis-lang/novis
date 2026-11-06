Starts a spreadsheet that your program writes to a file at a path.

A path that ends in `.csv` makes a CSV file, `.ods` makes an OpenDocument file (ods), and any other
path makes an Excel file (xlsx). The workbook writes to a temporary file in the same folder.
`finish` gives it the name `$path` and returns a `Written` with the row count of each sheet and the
file size. Its `bytes` is `null`.

**Good to know:** if a write fails, the temporary file is deleted and the file at `$path` does not
change. On a server, a program needs the `fs.write` capability in `nvs.toml` for the folder of the
path. A program you start from the command line may write any path.

A CSV file is UTF-8 with `,` between values. It starts with a byte order mark and ends each line
with CR and LF, so Excel opens it correctly. `csv` sets its `separator`, `quote` and `charset`.

**The examples below** save an Excel file and read it back, save a CSV file, save two sheets with
the summary first, save a CSV file with `;` in Windows-1252, and save an ods file.
