Reads every row of a query at once, as an array.

`Core\Db\Rows::all` returns one entry per row of the result. Each entry is a `Core\Db\Row`, and the
entries come in the order the server sent them. A query that matched no row returns an empty array.

The rows were already read from the database when `query` returned. This builds one small object per
row and reads nothing again, so you can call it as often as you like, and every call gives you the
same rows.

Where you wrote `Core\Db\Connection::queryAs` with a class of your own, each entry is an object of
that class.

**The examples below** read every row at once, read the rows of one query more than once, and hand a
whole result set to another method.
