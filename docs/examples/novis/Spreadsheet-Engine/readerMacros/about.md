Returns the names of the macro modules in a workbook.

`Engine::readerMacros` takes a `Reading` and its source. An xlsm or xls file can have macros,
which are programs written in VBA. The result is the name of each module that has them. The list
is empty when the workbook has no macros. Reading the names never runs a macro.

A program can check this list and not accept an upload that has macros. The source is the one
`Engine::readerOpen` read. For a workbook with no macros, the call does not read the source, so it
returns an empty list for any source.

**Good to know:** most programs do not call `Engine::readerMacros` directly. `Reader::macros` calls
it once and returns the same list.
