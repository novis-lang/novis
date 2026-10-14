Returns the cells of one column of the current sheet, such as `C2:C1043`.

`Engine::writerColumn` takes a `Writer` and a column. The column counts from 0, so column `A` is
0. The range starts at the first row after the header row and ends at the last row written so
far. A formula such as `SUM` can use it to add up the column. A sheet with no rows below its
header still returns one cell, the row after the header.

A column past the last one, `XFD`, throws a `LogicError`. So does a call when no sheet is open, and
a call after the workbook is finished.

**Good to know:** most programs do not call `Engine::writerColumn` directly. `Sheet::column` finds
the column of a key, such as `price`, sends the rows it has, and calls it once.
