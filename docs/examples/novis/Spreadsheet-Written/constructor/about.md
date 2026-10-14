Makes a `Written`, the result of writing a spreadsheet, from its three values.

`new Written` takes `rows`, `size` and `bytes`, in that order. Each value becomes a read-only property
of the same name. `rows` has the number of rows of each sheet, by sheet name, without the header row.
`size` is the length of the file in bytes. `bytes` is the whole file from `createInMemory`. For a
download or a file, `bytes` is `null`.

**Good to know:** `Workbook::finish` makes a `Written` for you, and so do `save` and `to` of an
export. You write `new Written` yourself only when you need one without writing a file, for example
in a test of your own code.
