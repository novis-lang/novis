Computes every formula in the workbook, and returns the cells it could not compute.

A file saves a result next to each formula, and that result can be missing or wrong. After
`evaluate`, `rows` gives each formula cell the result that `evaluate` computed, with the cell's
number format.

Each entry in the returned list has `cell`, such as `Sheet1!B4`, and `kind`. `kind` is
`'function'` for a function that `evaluate` does not compute, such as `TODAY`. It is `'reference'`
for a reference to another file or a range used as one value. It is `'circular'` for a formula that
reads its own result, and `'token'` for formula text it cannot read. It is `'coercion'` for a
text such as `'$1'` or `'1/15/2024'` that a formula uses as a number. Such a cell keeps the
result that is saved in the file.

**Good to know:** reading a file never computes anything until you call `evaluate`. It reads no
other file and no URL. A CSV file has no formulas, so the list is empty.

**The examples below** check a report's totals, find the cells a workbook cannot compute, and read
a formula with its computed result.
