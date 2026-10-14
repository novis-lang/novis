Opens a template: an Excel workbook (xlsx or xlsm) with placeholders such as `{{customer.name}}` in
its cells.

`$source` is the bytes of the file, or a `Core\IO\File` that your program opened for reading. The
bytes can come from an upload. `Template::open` checks the file and returns a `Template`. Then you
call `set` and `fill`, and choose where the file goes with `save`, `to`, `bytes` or `download`. A
template can be filled once, so open it again for each file you make.

**Good to know:** a file that is not an Excel workbook throws an error. The option `zone`, such as
`'Europe/Berlin'`, is the time zone for an `Instant` value. Without `zone`, an `Instant` throws an
error. An xlsm template gives an xlsx file, because the macros are not copied.

**The examples below** open a template file, fill the same template for each customer, and write an
`Instant` with a time zone.
