Reads one column of a query as a plain list.

`Core\Db\Rows::column` returns the value that one column holds in every row. There is one entry per
row, and the entries come in the same order as the rows.

Name the column with a string, or give its position as a whole number. The first column of the query
is at position 0. A position is the way to read a column with no name of its own, such as the
`count(*)` of a counting query.

A name that is no column of the result throws a `LogicError`. So does a position below zero or past
the last column. A result with no rows returns an empty array for any name, because it describes no
column for the name to be wrong about.

Each entry is the value the row already holds, so a column of large values costs no copy.

**The examples below** read one column from every row, name a column by its position, and answer a
request with one column as JSON.
