Starts an area chart and returns it. An area chart is a line chart with the space under each line
filled. Add each series with `series`, then put the chart on a sheet with `Sheet::chart`.

It takes the same options as `Chart::column`: `title`, `xTitle`, `yTitle`, `legend` and `labels`.
The `color` of a series is the color of its area.

**Good to know:** `combine` can draw an area on the same axes as columns or a line, such as a
target with the real sales. Two area charts do not combine. Add a second series to one area chart
instead.

**The examples below** draw the orders of each quarter, draw columns and an area, and show the
error for two area charts combined.
