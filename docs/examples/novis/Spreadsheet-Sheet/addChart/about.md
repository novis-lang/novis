Puts the chart `$chart` at the cell `$cell`, and returns the sheet. You build the chart with
`Chart`, such as `Chart::column()`, and add its series with `withSeries`.

The top-left corner of the chart is at the cell. `width` and `height` are its size in pixels. The
default size is 480 by 288 pixels.

A series range is a range of cells in this workbook, such as `B2:B9` or `'Shop orders'!B2:B9`. A
range in another file, a web address and a range over more than one sheet throw a `LogicError`. A title
and an axis title have at most 255 characters.

**Good to know:** give each series a range from `$sheet->column($key)`. The chart then also
shows the rows you add after it. The file stores only the ranges of the chart. Excel reads the
values from the cells. A CSV file has no charts, so `addChart` does nothing there.
