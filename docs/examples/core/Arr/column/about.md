Takes one named value out of every row and returns those values as one array.

Rows are arrays: the rows a database query returned, the lines of a CSV file, a list of records.
`Core\Arr::column` gives you the value of the same key in each row, in the order of the rows. It
replaces PHP's `array_column`. A row that does not have that key is skipped.

You can also key the result by a second value. Write `indexBy` with the key you want, and each
value lands under the value of that cell. Two rows with the same key give one entry, and the last
row wins.

**The examples below** show one value from every row, a list where some rows do not have the key,
and a lookup table built with `indexBy`.
