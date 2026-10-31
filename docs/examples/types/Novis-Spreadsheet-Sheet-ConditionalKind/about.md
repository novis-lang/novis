The kind of a conditional format, a rule that changes how Excel shows the cells of a range.

You set it with the option `kind` of `Sheet::conditional`. The method turns the string into a
`Sheet::ConditionalKind` case:

- `'cell'` is `Cell`. It compares each cell with `values`, using `operator`.
- `'formula'` is `Formula`. A cell matches when the formula in `values` is true.
- `'colorScale'` is `ColorScale`. It colors each cell by its value, from the `colors` you give.
- `'dataBar'` is `DataBar`. It draws a bar in each cell, as long as its value.
- `'iconSet'` is `IconSet`. It shows an icon from the set `icons` in each cell.
- `'top'` and `'bottom'` are `Top` and `Bottom`. They match the `rank` highest or lowest values.
- `'duplicates'` and `'unique'` are `Duplicates` and `Unique`.

`Cell`, `Formula`, `Top`, `Bottom`, `Duplicates` and `Unique` need a `style` for the cells that match.
`ColorScale`, `DataBar` and `IconSet` do not take a `style`. Any other string throws a `LogicError`.
