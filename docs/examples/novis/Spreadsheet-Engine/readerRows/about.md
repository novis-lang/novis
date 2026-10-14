Returns the next rows of a sheet, at most `max` of them.

`Engine::readerRows` takes a `Reading`, its source, the read options and `max`. The options name the
sheet, the first row in `from`, the columns, and which columns are read as the text Excel shows.
They also say whether to read formulas, keep empty rows, fill merged cells and skip hidden rows.
Each row has its `number` and its cells. Each cell has its `column`, its value, and its formula and
shown text when the options turn them on.

`done` is `true` after the last row of the sheet. To read the rows after a batch, call it again with
`from` one past the last row number. With `max` 0, the result has no rows and `done` is `false`. A
sheet name the workbook does not have throws a `LogicError`.

**Good to know:** most programs do not call `Engine::readerRows` directly. `Reader::rows` calls it
for 1,000 rows at a time, and gives one `Row` at a time.
