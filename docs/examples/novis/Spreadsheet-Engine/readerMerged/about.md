Returns the merged ranges of one sheet.

`Engine::readerMerged` takes a `Reading`, its source and the name of a sheet. Each merged range is
text such as `A1:C2`. The list is empty when the sheet has no merged cells. When rows are read,
the value of a merged range is in its top-left cell.

A sheet name the workbook does not have throws a `LogicError`. So does a source that is not `bytes`
or a file. The source is the one `Engine::readerOpen` read. Another source can throw a `ParseError`
or give wrong results.

**Good to know:** most programs do not call `Engine::readerMerged` directly. `Reader::merged`
calls it once, and also finds a sheet by its position.
