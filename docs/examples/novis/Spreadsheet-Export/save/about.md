Writes the spreadsheet to a file at `$path`.

The file is created, or replaced if it exists. A path that ends in `.csv` makes a CSV file, and any
other path makes an Excel file (xlsx). `save` returns a `Written`: its `rows` has the number of rows
of each sheet, and its `size` is the length of the file in bytes. Its `bytes` is `null`, because the
file is on the disk.

**Good to know:** your program needs the `fs.write` capability in `nvs.toml` for the folder of the
path. Without it, `save` throws an error and writes nothing.
