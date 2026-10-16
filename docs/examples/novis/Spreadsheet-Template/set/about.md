Writes one value into one cell of a template, by the cell's address.

The address is the sheet name, then `!`, then the cell, such as `'Summary!B2'`. The cell does not
need a placeholder. `set` writes the value with the same types as `fill`: text, a number, a boolean,
a date or time from `Core\Time`, `null` or a `Formula`. A string that starts with `=` is written as
text. Use `Formula::of` to write a formula. `set` returns the template, so you can call it several
times in a row and then call `fill`.

**Good to know:** call `set` before `fill`. An address that names a sheet the template does not have
throws a `LogicError`.

**The examples below** write a title into a cell, fill a summary sheet beside an item list, and
write a value of each type.
