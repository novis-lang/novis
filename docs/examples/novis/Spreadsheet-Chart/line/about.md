Starts a line chart and returns it. Each series is one line that joins its values in the order of
the rows. Add each series with `series`, then put the chart on a sheet with `Sheet::chart`.

It takes the same options as `Chart::column`: `title`, `xTitle`, `yTitle`, `legend` and `labels`.
The `color` of a series is the color of its line.

**Good to know:** a line chart is good for values over time, such as visitors per day. `combine`
can draw the line on the same axes as columns or an area. A chart with no series throws a
`LogicError` when you put it on a sheet.

**The examples below** draw the visitors of each day, draw two lines with the legend at the bottom,
and show the error for a chart without a series.
