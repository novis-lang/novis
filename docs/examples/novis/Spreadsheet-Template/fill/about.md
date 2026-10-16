Fills the placeholders of a template with your data and returns an `Export`.

A placeholder is text such as `{{month}}` or `{{customer.name}}` in a cell of the template. Each dot
is one level of the array you pass. A cell that has only a placeholder gets the type of the value: a
number, a date, a boolean or text. The cell keeps its style from the template. A row with a list
placeholder, such as `{{items.qty}}`, is written once for each item of the list `items`. Every row
below it moves down, and a formula such as `SUM(D3:D3)` grows to cover all the items.

**Good to know:** a placeholder that your data does not have throws a `LogicError`. The error names
the cell and the placeholder. The error can come from `fill` or from the call that writes the file,
so put both in the `try` block. A value of `null` is allowed and leaves the cell empty. You can call
`fill` once on each template, and a list inside an item throws an error.

**The examples below** fill a monthly report with a list of items, fill values from nested arrays,
and show the errors.
