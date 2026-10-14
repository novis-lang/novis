Returns the names in the header row of a sheet, one for each column from `A`.

`Engine::readerHeader` takes a `Reading`, its source and the read options. `headerRow` is the row
number of the header. With `null`, the header is the first row at or after `from` that has a value.
A name is the text of its cell. An empty cell gets its column letter, such as `C`. A name that is
already used gets ` (2)`, ` (3)` and so on after it.

The result is empty when the header row is past the last row. A sheet name the workbook does not
have throws a `LogicError`, and so does a source that is not `bytes` or a file.

**Good to know:** most programs do not call `Engine::readerHeader` directly. `Reader::rows` with
`header: true` or a row number calls it once, and uses the names as the keys of each `Row`.
