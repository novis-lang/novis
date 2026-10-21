Makes a chart from all of its values. You do not usually call `new Chart` in your program:
`Chart::column`, `Chart::line` and the other types make the chart and return it.

The properties of a chart are read-only. `kind` is the type of the chart, and `series` is the list of
its series. `combined` is the list of the types and series that `combine` added. `title`, `xTitle`,
`yTitle`, `legend` and `labels` are the options of the chart.

**Good to know:** a chart does not change. `series` and `combine` return a new chart with one more
part, so keep the chart they return.

**The examples** of `Chart::column` show how to start a chart, and the examples of `series` show
how to add its data.
