Makes an `Export`, a whole spreadsheet that is ready to write, from the function that adds its rows.

`new Export` takes a function and the options of the workbook. The function gets a new `Workbook`
and adds the sheets and rows to it. `new Export` only stores these values. The file is written when
you call one method of the export: `download`, `save`, `to` or `bytes`.

**Good to know:** `Workbook::fromRows` and `Template::fill` make an `Export` for you, and most
programs use one of them. You write `new Export` yourself when you want the same short call for a
workbook with more than one sheet.
