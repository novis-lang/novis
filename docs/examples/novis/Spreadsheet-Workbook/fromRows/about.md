Makes a spreadsheet with one sheet from a list of rows.

`$rows` is an array of rows, or a generator that gives one row at a time. Each value in a row is one
cell. When the rows have keys, the sheet's first row shows them.

`fromRows` returns an `Export`. Call one of its methods to write the file. `download` sends it to the
browser, and `save` writes it to a path. `to` writes it to an open file, and `bytes` returns it.

It takes the options of `createInMemory` and of `sheet`. `format` is `xlsx`, `ods` or `csv`. Without
it, `download` and `save` take the format from the file name, and the others write xlsx. For CSV,
`csv` sets the `separator`, the `quote` and the `encoding`.

**Good to know:** a generator can be read only once, so a second write of its `Export` throws a
`LogicError`. For more sheets, use a `create` method and call `sheet` for each one.
