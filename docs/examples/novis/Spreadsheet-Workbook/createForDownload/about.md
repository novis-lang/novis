Starts a spreadsheet that your program sends to the browser as a file to download.

`$name` is the file name the browser saves. A name that ends in `.csv` makes a CSV file, and any other
name makes an Excel file (xlsx). `format: "csv"` or `format: "xlsx"` sets the format whatever the
name is. The response headers are sent at once. After that, each part of the file is sent as soon as
it is ready, so a large export does not have to fit in memory.

Add a sheet with `sheet`, add rows to it, and call `finish` at the end.

**Good to know:** set any other response header before you call `createForDownload`, because it sends
the headers. In a CSV file, a value that starts with `=`, `+`, `-` or `@` gets a `'` in front. A
spreadsheet program then shows it as text and does not run it as a formula.
