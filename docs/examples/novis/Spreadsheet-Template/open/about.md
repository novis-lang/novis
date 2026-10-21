Opens a template: an Excel workbook (xlsx or xlsm) with placeholders such as `{{customer.name}}` in
its cells.

`$source` is the bytes of the file, or a `Core\IO\File` that your program opened. `Template::open`
checks the file and returns a `Template`. Then you call `set` and `fill`, and choose where the file
goes with `save`, `to`, `bytes` or `download`. Open the template again for each file you make.

**Good to know:** the filled file copies the formulas and links of the template without a change.
For this reason, `$source` cannot be `tainted`, such as an uploaded file. A file that is not an Excel
workbook throws an error. The option `zone`, such as `'Europe/Berlin'`, is the time zone for an
`Instant` value. An xlsm template gives an xlsx file, because the macros are not copied.

**The examples below** open a template file, fill the same template for each customer, and write an
`Instant` with a time zone.
