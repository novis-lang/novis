Where a chart shows its legend, the list of series names and their colors.

You set it with the option `legend` of `Chart::column` and the other methods that make a chart. That method
converts the string to a `Chart::Legend` case:

- `'top'`, `'right'`, `'bottom'` and `'left'` are `Top`, `Right`, `Bottom` and `Left`.
- `'hidden'` is `Hidden`. The chart has no legend.

Without the option, the legend is `Right`. Any other string throws a `LogicError`. The property
`legend` of a `Chart` is one of these cases.
