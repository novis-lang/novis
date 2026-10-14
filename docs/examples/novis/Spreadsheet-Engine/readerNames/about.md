Returns the defined names of a workbook.

`Engine::readerNames` takes a `Reading` and its source. Each defined name has its `name` and its
`reference`, the cells the name points to, such as `Shop!$A$1:$C$9`. `sheet` is `null` for a name
the whole workbook can use. For a name that only one sheet can use, it is the name of that sheet.
The reference is returned as text, and nothing calculates it.

A source that is not `bytes` or a file throws a `LogicError`. The source is the one
`Engine::readerOpen` read. Another source can throw a `ParseError` or give wrong results.

**Good to know:** most programs do not call `Engine::readerNames` directly. `Reader::names` calls
it once and returns the same list.
