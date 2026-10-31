Gives a cell or a range of cells a name, so a formula can use the name instead of the address.

`$name` is the name, such as `Totals`. It starts with a letter or `_`, and has only letters, digits,
`_` and `.`. It has at most 255 characters. A name that looks like a cell, such as `C3` or `R1C1`,
is not allowed, and each name can be used only once. Excel does not see a difference between
capital and small letters here.

`$reference` is the cells the name points to, such as `Orders!$B$2:$B$9`. It is a sheet name, `!`
and a cell or a range. A range can also be whole columns, such as `Orders!$A:$C`, or whole rows,
such as `Orders!$1:$3`. Put a sheet name with a space or another sign in single quotes:
`'Shop orders'!$A$1`. The reference must point into this workbook. Another file, a web address and
a formula are not allowed.

A name or a reference that breaks these rules throws a `LogicError`. A name whose sheet is not in
the workbook throws a `LogicError` when you call `finish()`.

`define` returns the workbook, so you can define several names in a row. The whole workbook can use
each name.

**Good to know:** the names are written at the end of the file, so you can define a name before or
after the sheet and the rows it points to.
