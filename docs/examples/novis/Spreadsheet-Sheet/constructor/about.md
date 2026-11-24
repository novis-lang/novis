Makes one sheet of a workbook. You do not call `new Sheet` in your program: `Workbook::addSheet`
makes the sheet and returns it.

Every row you add goes to the file of the workbook that made the sheet. The rows are written in the
order you add them.

**Good to know:** a sheet is finished when the workbook starts the next sheet or when you call
`Workbook::finish`. After that, `addRow` and the other methods of the sheet throw a `LogicError`.

**The examples** of `Workbook::addSheet` show how to start a sheet, and the examples of `addRow` show
how to add its rows.
