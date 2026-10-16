Reads a CSV document and gives you its records, each one an array of text fields.

`Core\Csv::parse` takes the whole document as a string. Each record becomes one array, holding its
fields in the order they are written. A field in quotes may contain the separator, a line break, or
a quote written twice, and you get that text back without the quotes around it. A record ends with a
newline, or with a carriage return and a newline.

Set `header` to `true` and the first record becomes the column names. Every later record is then
keyed by those names, and the header itself is not part of the result. The `separator`, `quote` and
`escape` options read a document written with other characters.

**Good to know:** a record with fewer fields than the header keeps only the fields it has, and
nothing is filled in for the rest. A record with more fields keys the extra ones by their position.
