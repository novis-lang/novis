Makes one sheet of a workbook. You do not call `new Sheet` in your program: `Workbook::sheet` makes
the sheet and returns it.

The constructor stores the position of the sheet in the workbook, and the functions of the workbook
that write the rows and the other parts of the sheet. So every row you add goes to the file of the
workbook that made the sheet, in the order you add it.

**Good to know:** a sheet is finished when the workbook starts the next sheet or when you call
`Workbook::finish`. After that, `add` and the other methods of the sheet throw a `LogicError`.

**The examples** of `Workbook::sheet` show how to start a sheet, and the examples of `add` show how
to add its rows.
