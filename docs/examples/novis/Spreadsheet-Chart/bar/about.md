Starts a bar chart, a chart of horizontal bars, and returns it. Add each series with `series`, then
put the chart on a sheet with `Sheet::chart`.

It takes the same options as `Chart::column`: `title`, `xTitle`, `yTitle`, `legend` and `labels`.
A bar chart is good for long labels, such as the names of shops or products, because each label
has a full line beside its bar.

**Good to know:** a bar chart does not combine with other types. `Sheet::chart` throws a
`LogicError` for a bar chart with `combine`. To draw a line over bars, use `Chart::column`.

**The examples below** draw the sales of each shop, draw two years side by side, and show the error
for bars combined with a line.
