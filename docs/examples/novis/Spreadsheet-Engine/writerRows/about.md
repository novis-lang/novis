Adds rows to the current sheet and returns the bytes of the file that are ready to send.

`Engine::writerRows` takes a `Writer` and a list of rows. Each row has its cells, a style number,
a height, whether it is hidden, its outline level and whether it is the header row. Each cell has
a value and a style number. A value is text, a number, a whole number, a decimal, a true or false
value, a date, a time, a duration, a date with a time, a formula or rich text. Text is always
written as text, so text that looks like XML or like a formula stays as it is.

No open sheet, a row with more than 16,384 cells, a height outside 0 to 409 and an outline level
over 7 throw a `LogicError`. So do a style number that `Engine::writerStyle` did not return, and a
row past 1,048,576 when the sheet options do not start a new sheet.

**Good to know:** most programs do not call `Engine::writerRows` directly. `Sheet::add` and
`Sheet::addAll` collect rows, and call it with 1,000 rows at a time.
