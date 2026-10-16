Locks the sheet, so a person cannot change its cells or its layout in Excel, and returns the sheet.

`password` is the password a person types in Excel to unlock the sheet. Without it, anyone can unlock
the sheet with one click. `allow` names what a person can still do on a locked sheet, such as
`"sort"`, `"autoFilter"`, `"formatColumns"` or `"insertRows"`. A name that is not on that list
throws a `LogicError`.

**Good to know:** protection is not a security feature. The file keeps only a short hash of the
password, and any tool can remove it. Every program can still read the values of a locked sheet. Use
protection to stop mistakes, and never to hide data. A CSV file has no protection, so `protect` does
nothing there.

**The examples below** lock the layout of a price list, allow sorting and filters, and read the
values of a locked sheet.
