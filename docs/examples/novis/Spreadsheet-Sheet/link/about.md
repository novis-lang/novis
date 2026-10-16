Makes the cell `$cell` a link (a hyperlink) to `$target`, and returns the sheet.

The target is an `http:` or `https:` address, a `mailto:` address, or a cell of the workbook, such as
`"Totals!A1"`. `text` is the text the cell shows. `tooltip` is the text a person sees when the mouse
is over the link.

**Good to know:** any other target, such as a `file:` or `javascript:` address, throws a
`LogicError`. So a link built from user data cannot open a local file or run a script. Put the name
of a sheet with a space in quotes, such as `"'Sheet 2'!A1"`. A CSV file has no links, so `link` does
nothing there.

**The examples below** link to a website, link to a cell on another sheet, and show the error for a
target that is not allowed.
