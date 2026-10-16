Makes a style for cells: the font, the colors, the borders, the alignment and the number format.

Every field is optional. A field you leave out comes from the row, then from the column, and then
from Excel's default. `bold`, `italic` and `wrap` are `true` or `false`. `color` and `fill` are
colors such as `"#FF0000"`. `border` is a line such as `"thin"`, for every side or for each side
on its own. `format` is a number format, such as `"0.00"`, or a short name such as `"money"`,
`"percent"` or `"date"`.

Give a style to a cell with `Cell::of`, to a row with the `style` option of `add`, or to a column
with `columns`. Two styles with the same fields are one style in the file.

**Good to know:** a value that is not allowed, such as `border: "wavy"`, throws a `LogicError` with
the values that are allowed. A CSV file has no styles, so they are left out.
