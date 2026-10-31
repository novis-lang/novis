Returns the links (hyperlinks) of one sheet.

`$sheet` is the sheet name, or its position from 0. Each link has `anchor`, the cell of the link,
such as `"B2"`, and `target`, where the link goes. `text` is the text the cell shows, and `tooltip`
is the text a person sees over the link. Both are `null` when the link does not have them.
`external` is `true` for a URL or a file, and `false` for a cell in the same workbook, such as
`"Totals!A1"`.

**Good to know:** reading a file never opens its links. Every target comes from the file, so it is
`tainted`. Check it before your program shows it on a web page or follows it. A CSV file has no
links, so the list is empty. A sheet that does not exist throws a `LogicError`. `links` reads xlsx,
xlsm and ods files. For an xlsb or xls file, it throws an error.

**The examples below** list the links of a sheet, tell a website link from a link to a cell, and
check a link before a program uses it.
