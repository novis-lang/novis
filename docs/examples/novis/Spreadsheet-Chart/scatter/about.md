Starts a scatter chart, a chart of points, and returns it. Each row is one point. The `categories`
of a series are the x values of its points, and the series values are the y values. Add each
series with `series`, then put the chart on a sheet with `Sheet::chart`.

It takes the same options as `Chart::column`: `title`, `xTitle`, `yTitle`, `legend` and `labels`.
The `color` of a series is the color of its points.

**Good to know:** give every series `categories`, so each point has an x value. The chart reads
the x values as numbers, so use a column of numbers. A scatter chart does not combine with other
types.

**The examples below** draw the price of each product against the units sold, draw two groups of
points in two colors, and show the error for a scatter chart combined with a line.
