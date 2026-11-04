Writes rows of text as one CSV document, the format every spreadsheet program can open.

You give `Core\Csv::format` an array of rows. Each row is an array of strings, and its fields are
written in that order. A field that has a comma, a quote or a line break is put in quotes. So is a
field that starts or ends with a space, because many programs remove such a space. A quote inside a
quoted field is written twice. You never have to quote anything yourself. Every record ends with a
newline.

The `header` option writes column names as the first record. The `separator` and `quote` options set
the characters used, for a program that expects something other than a comma and `"`. The `lineEnd`
option is `"\n"` by default. Set it to `"\r\n"` for a program that needs CR and LF at the end of each
record.

**Good to know:** the keys of a row are ignored. Only its values are written, in the order they have.
`Core\Csv::parse` reads back exactly the fields you wrote.
