Sets how the sheet is printed, and returns the sheet.

`orientation` is `"portrait"` or `"landscape"`. `fitToPage` sets how many pages wide (`width`) and
tall (`height`) the sheet is printed on, and `0` means as many as needed. `margins` are in inches.
`header` and `footer` are the text at the top and the bottom of each page. `repeatRows`, such as
`"1:1"`, prints those rows at the top of every page. `area` is the range to print.

**Good to know:** set `fitToPage` before the first row of the sheet. After that, it throws a
`LogicError`. Give all the options in one call. A second `print` call on the same sheet throws a
`LogicError`. Any other `orientation` throws a `LogicError`. A CSV file has no page setup, so `print` does
nothing there.

**The examples below** print a wide report on one page, repeat the header row on every page, and
show the error for a fit set after the first row.
