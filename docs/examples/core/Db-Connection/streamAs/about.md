Runs one statement that reads rows, and gives them to you one at a time as objects of the class you
name.

`Core\Db\Connection::streamAs` works like `Core\Db\Connection::stream`, and it builds an object from
each row. You write the class in angle brackets, as in `streamAs<Person>`. Each round of a `foreach`
reads one row and builds one `Person`. Only the current object is in memory, so a table of ten
million rows uses about as much memory as a table of ten.

The class says how a row is read, as for `Core\Db\Connection::queryAs`: with the `#[Db\Derive]`
attribute or with a `fromRow` method.

The connection is busy until the last row is read. A second statement on it while rows remain throws
a `LogicError`. A loop you leave with `break` keeps the connection busy until the request ends, so put
a `limit` in the statement instead.

The examples read a table as objects, pass a typed stream to the rest of a program, and report on a
table that is larger than memory.
