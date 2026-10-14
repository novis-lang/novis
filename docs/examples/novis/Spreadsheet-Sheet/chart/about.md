Puts the chart `$chart` at the cell `$cell`, and returns the sheet. You build the chart with `Chart`,
such as `Chart::column()`, and add its series with `series`.

`width` and `height` are the size of the chart in pixels. Without them, the chart is 480 by 288
pixels. The top left corner of the chart is at the cell.

**Good to know:** give each series a range from `$sheet->column($key)`. That range grows with the
sheet, so a chart you add before the last row still shows every row. The chart is written when the
sheet is finished, and it contains no values of its own: Excel reads them from the cells. A CSV file
has no charts, so `chart` does nothing there.

**The examples below** write a report with a total row and a chart, draw a pie chart of the sales of
each shop, and show the error for two chart types that cannot be combined.
