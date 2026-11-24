Puts rows, such as `"5:9"`, or columns, such as `"B:D"`, in an outline group, and returns the sheet.
Excel shows a button beside the group, and a person can hide and show the group with one click.

`collapsed: true` hides the rows or columns of the group when the file opens. A person can still
show them in Excel.

**Good to know:** add a group of rows before you add the rows in it. Add a group of columns before the first row of the sheet. A group added too late
throws a `LogicError`. A row can also get its group from the `level` option of `addRow`. A CSV file
has no groups, so `addOutlineGroup` does nothing there.

**The examples below** group the order lines under each order, and hide a group of columns. The last
one shows the error for a group added after its rows.
