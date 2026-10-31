Makes a spreadsheet with one sheet from a list of rows, in one call.

`$rows` is an array of rows or a generator that gives one row at a time. Each row is an array, and
each value in it is one cell. When the rows have keys, the first row of the sheet shows the keys as
a header. `fromRows` returns an `Export`, and you call one method on it to write the file. `download`
sends it to the browser, and `save` writes it to a path. `to` writes it to an open file, and `bytes`
returns the whole file.

The options are the options of `createInMemory` and of `sheet` together. `sheet` is the name of the
sheet, and the default is `Sheet1`. For a CSV file, `csv` sets the `separator`, the `quote` and the
`encoding`.

**Good to know:** for more than one sheet, start with a `create` method and call `sheet` for each one.
