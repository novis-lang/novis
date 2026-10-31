Fills the placeholders of a template with your data and returns an `Export`.

A placeholder is text such as `{{month}}` or `{{customer.name}}` in a cell of the template. Each dot
is one level of the array you pass. A cell that has only a placeholder gets the type of the value,
and keeps its style. A row with a list placeholder, such as `{{items.qty}}`, is written once for
each item of the list `items`. A list is an array or an `Iterator`, such as a generator. Every row
below it moves down, and a formula such as `SUM(D3:D3)` grows to cover all the items. Each sheet
can have its own lists.

**Good to know:** a placeholder that your data does not have throws a `LogicError` that names the
cell. The error can come from `fill` or from the call that writes the file, so put both in the
`try` block. A value of `null` leaves the cell empty. A list inside an item throws an error.

**The examples below** fill a report with a list of items, fill values from nested arrays, show
the errors, and fill a list on each of two sheets.
