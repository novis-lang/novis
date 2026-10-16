Returns the sheets of the workbook, in the order of their tabs in Excel.

Each sheet has its `name`, its `position` from 0, and its `visibility`. `visibility` is
`Visibility::Visible`, `Visibility::Hidden` or `Visibility::VeryHidden`. A very hidden sheet is one
that a person cannot show again from the Excel menus. `kind` says whether the sheet is a worksheet or
a chart sheet. `rows` and `columns` are the size that the file states for the sheet. They are `0` when the file
does not state a size, so count the rows that `rows` gives when you need the real number.

**Good to know:** the name of a sheet comes from the file, so it is `tainted`. To read a sheet you
found here, pass its `position` to `rows`, `merged`, `links`, `notes`, `tables` or `images`. A CSV file
has one sheet, `Sheet1`.

**The examples below** list the sheets, find the hidden ones, and read every sheet one after
another.
