Computes every formula in the workbook, and returns the cells it could not compute.

A file saves a result next to each formula. That result can be missing or wrong, for example when
another program wrote the file. After `evaluate`, `rows` gives each formula cell the result that
`evaluate` computed. The result has the cell's number format, as any other cell has.

Each entry in the returned list has `cell`, such as `Sheet1!B4`, and `kind`. `kind` is
`'function'` for a function that `evaluate` does not compute, such as `TODAY`. It is `'reference'`
for a reference to another file or a range used as one value. It is `'circular'` for a formula that
reads its own result, and `'token'` for formula text it cannot read. Such a cell keeps the result
that is saved in the file.

**Good to know:** reading a file never computes anything until you call `evaluate`. It reads no
other file and no URL. A CSV file has no formulas, so the list is empty.

**The examples below** check a report's totals, find the cells a workbook cannot compute, and read
a formula with its computed result.
