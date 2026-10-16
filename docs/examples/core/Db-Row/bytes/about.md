Reads one column of a row as bytes.

`Core\Db\Row::bytes` takes the name of a column and returns the octets that are stored in it. It is
the reader for a `BINARY`, `BLOB` or `BYTEA` column, which is where a picture, a document or a hash
is kept. That data has no encoding, so it is not text.

`bytes` and `string` are separate types in Novis, and this method never converts between them. Read a
text column with `Core\Db\Row::string`.

The result is `null` when the column has no value in this row, so the type you get back is `?bytes`.
The method throws a `LogicError` when the row has no column with that name, and when the column holds
text rather than bytes.

**The examples below** measure a picture that is stored in a row, read a column that may have no
value, and find out which file type a stored picture is.
