Returns the merged cells of one sheet.

`$sheet` is the sheet name, or its position from 0. Each item in the list is a range, such as
`"A1:C1"`, from the top-left cell to the bottom-right cell of one merged area. A sheet without merged
cells gives an empty list, and so does a CSV file.

**Good to know:** in a file, only the top-left cell of a merged area has a value. When you read rows,
the other cells of the area are `null`. Read with `merged: 'fill'` to give every cell of the area the
value of the top-left cell. Each range is a `tainted string`, because it comes from the file. A
sheet that does not exist throws a `LogicError`.

**The examples below** list the merged cells, fill every cell of a merged area, and check each sheet
before an import.
