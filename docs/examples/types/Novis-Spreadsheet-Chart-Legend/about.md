Where a chart shows its legend, the list of series names and their colors.

You set it with the option `legend` of `Chart::column` and the other start methods. The method turns
the string into a `Chart::Legend` case:

- `'top'`, `'right'`, `'bottom'` and `'left'` are `Top`, `Right`, `Bottom` and `Left`.
- `'hidden'` is `Hidden`. The chart has no legend.

Without the option, the legend is `Right`. Any other string throws a `LogicError`. The property
`legend` of a `Chart` is its place.
