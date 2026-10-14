Returns the sheets of a workbook, in tab order.

`Engine::readerSheets` takes a `Reading` and its source. Each sheet has its `name`, its `position`
from 0, its `visibility`, and its `kind`, which is a sheet with cells or a chart sheet. `rows` and
`columns` are the size the file gives for the sheet, or 0 when it gives none. The file can give a
wrong size, so a program reads the rows to count them.

The source is the one `Engine::readerOpen` read. Another source can throw a `ParseError` or give
wrong results. A source that is not `bytes` or a file throws a `LogicError`.

**Good to know:** most programs do not call `Engine::readerSheets` directly. `Reader::sheets`
calls it once and returns the same list. `Reader::rows` and the other `Reader` methods use it to
find a sheet by its position.
