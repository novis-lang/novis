Starts a pie chart and returns it. Each value of the first series is one slice, and the
`categories` of the series are the names of the slices. Add the series with `series`, then put the
chart on a sheet with `Sheet::chart`.

It takes the same options as `Chart::column`. A pie chart has no axes, so `xTitle` and `yTitle`
are not shown. `labels: true` shows the value on each slice.

**Good to know:** give a pie chart one series. A pie chart does not combine with other types:
`Sheet::chart` throws a `LogicError` for a pie chart with `combine`. A range with a sheet name,
such as `Sales!B2:B3`, uses the cells of that sheet.

**The examples below** draw the sales of each shop, show the value on each slice, and draw a pie
of the rows of another sheet.
