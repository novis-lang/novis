Starts a new sheet in a workbook and returns it, so you can add rows to it.

`$name` is the name on the tab. It has at most 31 characters, cannot have `: \ / ? * [ ]` or a
control character, and cannot start or end with `'`. Excel keeps the name `History` for itself, so
it is not allowed. Each sheet needs its own name. A name that breaks these rules throws a
`LogicError`. The sheets are written one after another. When you start a new sheet, the
sheet before it is finished, and adding a row to it throws a `LogicError`. `position` sets where the
tab shows, so a summary written last can still be the first tab.

When the rows have keys, the first row shows the keys in bold, with a filter, and stays at the top
when a person scrolls. `header`, `headerStyle`, `freeze` and `autofilter` change this, and `columns`
sets the header text, the width and the format of each column.

**Good to know:** a CSV file has only one sheet, so a second `sheet` call throws a `LogicError`.
