Returns the defined names of the workbook. A defined name is a name for a cell, a range or a
value, such as `TaxRate` for `Shop!$B$2`.

Each name has `name`, `reference` and `sheet`. `reference` is the text of the name as Excel shows
it in the Name Manager, such as `"Shop!$A$1:$C$9"`. `sheet` is the name of the one sheet that can use
the name. It is `null` for a name that the whole workbook can use. Excel also keeps some names for
itself, such as `_xlnm._FilterDatabase` for the range of a filter.

**Good to know:** every value comes from the file, so it is `tainted`. `names` does not read the
cells of a reference. Pass the sheet to `rows` and choose the rows and columns there. A CSV file has
no names, so the list is empty. `names` reads xlsx, xlsm, xlsb and ods files. For an xls file, it
throws an error.

**The examples below** list the names, find one name, and read a name that belongs to one sheet.
