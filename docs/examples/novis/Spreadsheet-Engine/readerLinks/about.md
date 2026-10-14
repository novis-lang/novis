Returns the hyperlinks of one sheet.

`Engine::readerLinks` takes a `Reading`, its source and the name of a sheet. Each link has its
`anchor`, the cell such as `B2`, and its `target`, the place it goes. `text` and `tooltip` are the
text the link shows, or `null`. `external` is `true` for a link to a URL or a file, and `false` for
a link to a cell of the workbook. Reading a link never opens it.

A sheet name the workbook does not have throws a `LogicError`. So does a source that is not `bytes`
or a file. The source is the one `Engine::readerOpen` read. Another source can throw a `ParseError`
or give wrong results.

**Good to know:** most programs do not call `Engine::readerLinks` directly. `Reader::links` calls
it once, and also finds a sheet by its position.
