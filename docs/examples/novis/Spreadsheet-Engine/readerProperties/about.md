Returns the document properties of a workbook.

`Engine::readerProperties` takes a `Reading` and its source. The result has the `title`, the
`subject`, the `author`, the `company` and the `keywords` of the workbook. A property the file does
not have is `null`. `created` is the date and the time of day the workbook was created, in UTC, or
`null`.

An xls or ods file throws an error. A source that is not `bytes` or a file throws a `LogicError`.
Each call reads the properties part of the file again, and counts against the bound `maxBytes` of
`Engine::readerOpen`. Many calls on one `Reading` can throw a `ParseError` for that reason.

**Good to know:** most programs do not call `Engine::readerProperties` directly.
`Reader::properties` calls it once and turns `created` into a `Core\Time\Instant`.
