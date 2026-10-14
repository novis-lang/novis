Starts a doughnut chart and returns it. A doughnut chart is a pie chart with a hole in the middle.
Each value of the series is one part of the ring. Add the series with `series`, then put the chart
on a sheet with `Sheet::chart`.

It takes the same options as `Chart::column`. A doughnut chart has no axes, so `xTitle` and
`yTitle` are not shown. `labels: true` shows the value on each part.

**Good to know:** the hole is half the size of the doughnut. A doughnut chart does not combine
with other types: `Sheet::chart` throws a `LogicError` for a doughnut chart with `combine`.

**The examples below** draw the share of each shop, put the legend on the left, and show the error
for a doughnut combined with a pie.
