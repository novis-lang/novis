Returns the chart with the type and the series of `$other` drawn on the same axes. Use it for two
kinds of data in one chart, such as columns of the sales and a line of the costs. The chart you
call it on does not change.

The title, the axis titles, the legend and `labels` come from the chart you call `combine` on. The
options of `$other` are not used.

**Good to know:** a combined chart has two different types of column, line and area. `Sheet::chart`
throws a `LogicError` for any other combination, such as a pie with a line, two lines, or three
types.

**The examples below** draw columns of the sales with a line of the costs, show that the first
chart does not change, and show the error for three types in one chart.
