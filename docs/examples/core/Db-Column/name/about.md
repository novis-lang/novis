Gives the label of one column in a query result.

A query result describes its columns before you read any rows. `name()` returns the label the
database gave one column. When the query writes `as`, that alias is the label. When it does not, the
label is the column name from the table.

You can print the label as a heading, and you can hand it to a row to read that column. The
description belongs to the statement, not to a row, so the labels are there even when the query
matched nothing.

**Good to know:** two columns can carry the same label. `select id, id from notes` describes two
columns, and both are called `id`.
