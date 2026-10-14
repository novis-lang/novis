Returns the notes of one sheet.

`Engine::readerNotes` takes a `Reading`, its source and the name of a sheet. Each note has its
cell in `noted`, such as `A3`, its text in `note`, and its `author`, or `null` when it has none.
Text that looks like XML comes back as the same text.

A sheet name the workbook does not have throws a `LogicError`. So does a source that is not `bytes`
or a file. The source is the one `Engine::readerOpen` read. Another source can throw a `ParseError`
or give wrong results.

**Good to know:** most programs do not call `Engine::readerNotes` directly. `Reader::notes` calls
it once, and also finds a sheet by its position.
