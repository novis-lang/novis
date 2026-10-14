Puts one value in a cell of a template, by its address.

`Engine::templateSet` takes a `Filling`, an address such as `Summary!B2`, and a value. The value
is text, a number, a whole number, a decimal, a true or false value, a date, a time, a duration, a
date with a time, or a formula. It replaces what the cell had in the template. Text is always
written as text, so text that looks like XML stays as it is.

Call it before `Engine::templateFill`. A call after it throws a `LogicError`. So do an address that
is not one cell, such as a range, and a sheet the template does not have.

**Good to know:** most programs do not call `Engine::templateSet` directly. `Template::set` turns
a value of the program into a cell value and calls it once.
