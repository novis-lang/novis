Whether a sheet of a workbook has cells or only a chart.

- `Worksheet` is a sheet with rows and cells.
- `Chartsheet` is a sheet that is one big chart. It has no cells.

`Reader::sheets` gives the `kind` of each sheet in a file. A workbook that you write has only
worksheets. A file made in Excel can also have chart sheets.

`Reader::rows` on a chart sheet throws an error, because it has no rows. Check the `kind` first when
your program reads every sheet of an upload.
