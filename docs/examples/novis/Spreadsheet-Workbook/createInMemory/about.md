Starts a spreadsheet that your program keeps in memory, as an Excel file (xlsx) or a CSV file.

Add a sheet with `sheet`, add rows to it, and call `finish` at the end. `finish` returns a `Written`.
Its `bytes` is the whole file, and its `size` is the length of the file in bytes. Its `rows` has the
number of rows of each sheet. `format: "csv"` makes a CSV file. The other options set the time zone
for dates and the document properties, such as `title` and `author`.

**Good to know:** the whole file is in memory until your program is done with it. For a large export,
`createForDownload` or `createForFile` sends each part of the file as soon as it is ready.

**The examples below** make an Excel file and read it back, make the same rows as CSV, and compare a
report with a saved copy.
