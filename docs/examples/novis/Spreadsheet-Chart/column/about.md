Starts a column chart, a chart of vertical bars, and returns it. The chart has no data yet: add each
series with `series`, then put the chart on a sheet with `Sheet::chart`.

Every option is optional. `title` is the title of the chart. `xTitle` is the title under the bars,
and `yTitle` is the title beside them. `legend` is `'right'`, `'top'`, `'bottom'`, `'left'` or
`'hidden'`, and the default is `'right'`. `labels: true` shows the value of each bar.

**Good to know:** a `legend` with other text throws a `LogicError`. A chart does not change: each
method returns a new chart. `combine` can draw a line or an area on the same axes as the columns.

**The examples below** draw the sales of each month, add axis titles and values, and show the error
for a legend that is not allowed.
