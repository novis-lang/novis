Gives a cell or a range of cells a name, so a formula can use the name instead of the address.

`$name` starts with a letter or `_`, and has only letters, digits, `_` and `.`. It has at most 255
characters. It cannot look like a cell, such as `C3` or `R1C1`. Each name can be used once.

`$reference` is a sheet name, `!` and a cell or a range, such as `Orders!$B$2:$B$9`. It can also be
whole columns or rows, such as `Orders!$A:$C` or `Orders!$1:$3`. Put a sheet name with a space or
another sign in single quotes: `'Shop orders'!$A$1`. Another file, a web address and a formula are
not allowed.

A name or a reference that breaks these rules throws a `LogicError`. If its sheet is not in the
workbook, `finish()` throws a `LogicError`.

**Good to know:** you can define a name before or after its sheet, and the whole workbook can use it.
