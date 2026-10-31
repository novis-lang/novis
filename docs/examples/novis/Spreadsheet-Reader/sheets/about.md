Returns the sheets of the workbook, in the order of their tabs in Excel.

Each sheet has its `name`, its `position` from 0, and its `visibility`. `visibility` is
`Visibility::Visible`, `Visibility::Hidden` or `Visibility::VeryHidden`. A very hidden sheet is one
that a person cannot show again from the Excel menus. `kind` says whether the sheet is a worksheet or
a chart sheet. `rows` and `columns` are the size that the file states for the sheet. When the file
states no size, `sheets` counts the rows and columns of the sheet. `rows` is then the number of the
last row with a cell.

**Good to know:** the name of a sheet comes from the file, so it is `tainted`. You can pass the name
or the `position` to `rows`, `merged`, `links`, `notes`, `tables` or `images`. The reader reads the
sizes once and keeps them, so a second call to `sheets` is fast. A CSV file has one sheet, `Sheet1`.

**The examples below** list the sheets, find the hidden ones, and read every sheet one after
another.
