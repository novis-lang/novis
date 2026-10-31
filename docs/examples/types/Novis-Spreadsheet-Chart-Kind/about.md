The type of a chart.

Each case has a method of `Chart` that starts a chart of that type:

- `Column` is `Chart::column`, vertical bars.
- `Bar` is `Chart::bar`, horizontal bars.
- `Line` is `Chart::line`, the values of each series joined by a line.
- `Area` is `Chart::area`, a line chart with the space under each line filled.
- `Pie` is `Chart::pie`, and `Doughnut` is `Chart::doughnut`, a pie with a hole in the middle.
- `Scatter` is `Chart::scatter`, points with an x value and a y value.

The property `kind` of a `Chart` is its type. A chart made with `combine` keeps the type of the first
chart. You can combine two of `Column`, `Line` and `Area`.
