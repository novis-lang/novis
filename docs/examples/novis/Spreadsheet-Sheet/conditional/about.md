Adds one conditional format to the cells of `$range`, and returns the sheet. Excel changes the look
of each cell that matches the rule, and checks again when a value changes.

`kind` is `"cell"`, `"formula"`, `"colorScale"`, `"dataBar"`, `"iconSet"`, `"top"`, `"bottom"`,
`"duplicates"` or `"unique"`. A `"cell"` rule compares each value with `operator`, such as
`"greater"` or `"between"`, and `values`. `style` is the `Style` of the cells that match. A colour
scale, a data bar and an icon set take `colors` or `icons` instead of a `style`.

**Good to know:** `values` are Excel formula text, so a text value has quotes, such as `'"open"'`.
The rules of a sheet apply in the order you add them. A rule that needs a `style` and has none, or an
unknown `kind`, throws a `LogicError`. A CSV file has no formats, so `conditional` does nothing there.

**The examples below** mark low stock in red, colour the sales of each shop, and show the error for
a rule without a style.
