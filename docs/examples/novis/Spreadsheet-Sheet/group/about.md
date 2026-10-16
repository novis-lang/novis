Puts rows, such as `"5:9"`, or columns, such as `"B:D"`, in an outline group, and returns the sheet.
Excel shows a button beside the group, and a person can hide and show the group with one click.

`collapsed: true` hides the rows or columns of the group when the file opens. A person can still
show them in Excel.

**Good to know:** the rows of a group are written with the group, so add a group of rows before you
add its rows. Add a group of columns before the first row of the sheet. A group added too late throws
a `LogicError`. A row can also get its group from the `level` option of `add`. A CSV file has no
groups, so `group` does nothing there.

**The examples below** group the order lines under each order, hide a group of columns, and show
the error for a group added after its rows.
