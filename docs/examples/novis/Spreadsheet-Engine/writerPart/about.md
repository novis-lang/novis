Adds one part to the current sheet and returns the bytes of the file that are ready to send.

`Engine::writerPart` takes a `Writer` and one part. A part is a merged range, a link, a note, a
data validation, a row group, a table, a conditional format, a picture, a chart, the print setup
or the sheet protection. The shape of the part says which one it is. Most parts are written when
the sheet ends, and all their text is escaped.

A link goes to an `http:`, `https:` or `mailto:` address, or to a cell such as `'Sheet 2'!A1`.
Any other target throws a `LogicError`. So do a range that is not a range of cells, a merged range
that overlaps another one, a picture that is not an image, a table name Excel does not allow, and
a part when no sheet is open.

**Good to know:** most programs do not call `Engine::writerPart` directly. `Sheet::merge`,
`Sheet::link`, `Sheet::note`, `Sheet::validate`, `Sheet::group`, `Sheet::table`,
`Sheet::conditional`, `Sheet::image`, `Sheet::chart`, `Sheet::print` and `Sheet::protect` each
call it once.
