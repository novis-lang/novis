Returns the address of one cell, such as `C7`.

`Engine::writerCell` takes a `Writer`, a column and a row. The column counts from 0, so column
`A` is 0 and column `XFD`, the last one, is 16,383. The row counts from 1, and the last row is
1,048,576. The address has no sheet name and no `$`. A formula can use it to point at a cell of
the same sheet.

The result does not depend on the rows written so far. The call also works before the first sheet
and after the workbook is finished. A column or a row outside the sheet, and row 0, throw a
`LogicError`.

**Good to know:** most programs do not call `Engine::writerCell` directly. `Sheet::cell` finds the
column of a key, such as `price`, and calls it once.
