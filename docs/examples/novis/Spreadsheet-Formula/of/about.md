Makes an Excel formula for one cell, such as `SUM(C2:C9)`.

Write the formula the way you write it in Excel, with or without `=` at the start. An empty formula
throws a `LogicError`. A formula has at most 8,192 characters, as in Excel. A longer one throws a
`LogicError` when its row is written. Put the result
of `Formula::of` in a row in place of a value. Only a `Formula` is written as a formula. A string is
always written as text, even when it starts with `=`. So text that a user typed can never run as a
formula in the file.

`cached` is the result that the file has for the formula. A program that reads the file and does not
calculate formulas gets this value. Excel calculates every formula again when it opens the file.

**Good to know:** `$sheet->cell` and `$sheet->column` give the address of a cell and of a column,
so your program never needs to work out a column letter.
