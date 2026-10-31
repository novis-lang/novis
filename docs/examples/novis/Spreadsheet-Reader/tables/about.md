Returns the Excel tables of one sheet. An Excel table is a range of cells that a person made into a
table in Excel, with a name and a header row.

`$sheet` is the sheet name, or its position from 0. Each table has `name`, the table name such as
`"Prices"`, `table`, its range such as `"A1:C10"`, and `sheet`, the name of the sheet it is on. The
first row of the range is the header row of the table.

**Good to know:** `tables` does not read the cells. To read the rows of a table, pass its sheet to
`rows` with the header row as `header` and the count of rows as `limit`. Every value comes from the
file, so it is `tainted`. A CSV file has no tables, so the list is empty. A sheet that does not exist
throws a `LogicError`. `tables` reads xlsx, xlsm, xlsb and ods files. In an ods file, a table is a
database range. For an xls file, `tables` throws an error.

**The examples below** list the tables of a sheet, read the rows of one table, and find the tables of
every sheet.
