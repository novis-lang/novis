Gives a cell or a range of cells a name, so a formula can use the name instead of the address.

`$name` is the name, such as `Totals`. It starts with a letter or `_`, and has only letters, digits,
`_` and `.`. A name that looks like a cell, such as `C3`, is not allowed, and each name can be used
only once. Excel does not see a difference between capital and small letters here. `$reference` is
the cells the name points to, such as `Orders!$B$2:$B$9`. A name that breaks these rules throws a
`LogicError`.

`define` returns the workbook, so you can define several names in a row. The whole workbook can use
each name.

**Good to know:** the names are written at the end of the file, so you can define a name before or
after the rows it points to.
