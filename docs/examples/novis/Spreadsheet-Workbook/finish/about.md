Writes the end of the spreadsheet and returns a `Written` with what was written.

Call `finish` once, after the last row. It finishes the last sheet and writes the end of the file.
Then it sends that last part to the browser, the file or memory. A file from
`createForFile` is closed. A file from `createForStream` stays open.

The `Written` it returns has `rows`, the number of rows of each sheet by sheet name, and `size`, the
length of the file in bytes. The header row is not counted. `bytes` is the whole file for
`createInMemory`, and `null` for the other targets.

**Good to know:** a workbook that is never finished is not a valid file, and the program logs an
error for it. A workbook needs at least one sheet, and each call after `finish` throws a
`LogicError`.
